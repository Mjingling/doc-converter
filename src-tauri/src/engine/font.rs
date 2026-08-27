//! 嵌入式字体支持：从系统字体加载中文字体，子集化后构建 PDF CID 字体
//! （Type0 / CIDFontType2），使中文水印在不膨胀文件体积的前提下正常渲染。
//!
//! 标准 14 字体（Helvetica 等）只支持 Latin-1，无法渲染中文。本模块：
//! 1. 从 TTC/OTF/TTF 中正确提取单个 face（TTC 各 face 共享表数据，不能简单切片，
//!    需按表目录重建独立 sfnt）
//! 2. 对用到的字形做子集化（glyf 裁剪 + 复合字形递归 + cmap format4 重建），
//!    避免将整套中文字体（数十 MB）嵌入 PDF
//! 3. 构建 Type0 字体：Identity-H 编码（CID = Unicode 码点）+ ToUnicode CMap
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use std::collections::{BTreeSet, HashMap};

/// macOS 系统中文字体候选（按优先级；需为 TrueType 轮廓，CFF/OTTO 不支持 FontFile2）
#[cfg(target_os = "macos")]
const FONT_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/STHeiti Light.ttc",
    "/System/Library/Fonts/STHeiti Medium.ttc",
    "/System/Library/Fonts/PingFang.ttc",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
    "/System/Library/Fonts/Supplemental/Songti.ttc",
    "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
];

/// Windows 中文字体候选：微软雅黑 / 黑体 / 宋体 / 等线（客户端预装；
/// Server SKU（含 CI runner）可能均未装，此时水印返回错误属预期行为）
#[cfg(windows)]
const FONT_CANDIDATES: &[&str] = &[
    r"C:\Windows\Fonts\msyh.ttc",
    r"C:\Windows\Fonts\msyhbd.ttc",
    r"C:\Windows\Fonts\simhei.ttf",
    r"C:\Windows\Fonts\simsun.ttc",
    r"C:\Windows\Fonts\deng.ttf",
];

/// Linux 中文字体候选：文泉驿微米黑（TrueType 轮廓；
/// Noto Sans CJK 为 CFF/OTTO 轮廓，不适用于 CIDFontType2 的 FontFile2 嵌入）
#[cfg(all(unix, not(target_os = "macos")))]
const FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    "/usr/share/fonts/wqy-microhei/wqy-microhei.ttc",
    "/usr/share/fonts/truetype/wqy/wqy-zenhei/wqy-zenhei.ttc",
];

pub struct EmbeddedFont {
    /// PostScript 名称（用于 PDF /BaseFont）
    pub postscript_name: String,
    /// 单个 face 的完整 TrueType 数据（未子集化，用于度量查询）
    ttf_data: Vec<u8>,
    units_per_em: u16,
    /// 以下度量均换算为千分单位（PDF 字体度量惯例）
    pub ascent: i32,
    pub descent: i32,
    pub cap_height: i32,
    pub bbox: [i32; 4],
    /// 字符宽度缓存（千分单位）
    widths: HashMap<char, u16>,
}

/// 加载系统第一个可用的中文字体
pub fn load_system_font() -> Result<EmbeddedFont, String> {
    for path in FONT_CANDIDATES {
        let data = std::fs::read(path).unwrap_or_default();
        if data.is_empty() {
            continue;
        }
        if let Ok(font) = EmbeddedFont::from_data(path, &data) {
            return Ok(font);
        }
        eprintln!("[font] 解析失败，尝试下一个字体: {}", path);
    }
    Err("未找到可用的系统中文字体，无法渲染中文水印（Windows Server 等精简系统可能未预装中文字体）".to_string())
}

impl EmbeddedFont {
    /// 解析字体文件：TTC 取第一个 TrueType face（重建独立 sfnt），单 TTF 直接使用
    fn from_data(path: &str, data: &[u8]) -> Result<EmbeddedFont, String> {
        let ttf_data = if data.len() > 4 && &data[0..4] == b"ttcf" {
            // TTC 头：'ttcf' + version(4) + numFonts(4) + offsets[]
            if data.len() < 12 {
                return Err("TTC 文件头不完整".into());
            }
            let num = u32::from_be_bytes([data[8], data[9], data[10], data[11]]) as usize;
            if num == 0 || data.len() < 12 + num * 4 {
                return Err("TTC 字体数量异常".into());
            }
            // 依次尝试各 face，直到找到 TrueType（sfnt 1.0）轮廓的字体
            let mut extracted = None;
            for i in 0..num {
                let base = u32::from_be_bytes([
                    data[12 + i * 4],
                    data[13 + i * 4],
                    data[14 + i * 4],
                    data[15 + i * 4],
                ]) as usize;
                if base + 4 > data.len() || &data[base..base + 4] != &[0x00, 0x01, 0x00, 0x00] {
                    continue; // 跳过 CFF（'OTTO'）等非 TrueType face
                }
                if let Ok(ttf) = rebuild_face(data, base) {
                    extracted = Some(ttf);
                    break;
                }
            }
            extracted.ok_or("TTC 中无可用 TrueType face")?
        } else {
            data.to_vec()
        };

        let face = ttf_parser::Face::parse(&ttf_data, 0)
            .map_err(|e| format!("解析字体失败: {:?}", e))?;
        let units_per_em = face.units_per_em();
        if units_per_em == 0 {
            return Err("字体 unitsPerEm 为 0".into());
        }

        // PostScript 名称（name_id = 6），失败时用占位名
        let postscript_name = face
            .names()
            .into_iter()
            .filter(|n| n.name_id == ttf_parser::name_id::POST_SCRIPT_NAME)
            .find_map(|n| n.to_string())
            .unwrap_or_else(|| "DocMorphEmbedded".to_string());

        let scale = |v: i16| v as i32 * 1000 / units_per_em as i32;
        let bbox = face.global_bounding_box();
        let bbox = [
            bbox.x_min as i32 * 1000 / units_per_em as i32,
            bbox.y_min as i32 * 1000 / units_per_em as i32,
            bbox.x_max as i32 * 1000 / units_per_em as i32,
            bbox.y_max as i32 * 1000 / units_per_em as i32,
        ];
        let ascent = scale(face.ascender());
        let descent = scale(face.descender());
        let cap_height = face.capital_height().map(scale).unwrap_or(700);
        drop(face); // 度量提取完毕，释放对 ttf_data 的借用后才能移动它

        eprintln!(
            "[font] 加载 {} -> {} (upem={}, 子集化后体积将大幅缩小)",
            path, postscript_name, units_per_em
        );
        Ok(EmbeddedFont {
            postscript_name,
            ttf_data,
            units_per_em,
            ascent,
            descent,
            cap_height,
            bbox,
            widths: HashMap::new(),
        })
    }

    /// 字符宽度（千分单位），带缓存；缺字形时取半字宽兜底
    pub fn advance(&mut self, ch: char) -> u16 {
        if let Some(w) = self.widths.get(&ch) {
            return *w;
        }
        let w = ttf_parser::Face::parse(&self.ttf_data, 0)
            .ok()
            .and_then(|face| {
                face.glyph_index(ch)
                    .and_then(|gid| face.glyph_hor_advance(gid))
            })
            .unwrap_or(self.units_per_em / 2);
        let w1000 = (w as u32 * 1000 / self.units_per_em as u32) as u16;
        self.widths.insert(ch, w1000);
        w1000
    }

    /// 文本总宽度（千分单位）
    pub fn text_width(&mut self, chars: &[char]) -> u32 {
        chars.iter().map(|c| self.advance(*c) as u32).sum()
    }

    /// 在文档中构建 Type0 中文字体对象组，返回 Type0 字体对象 id
    pub fn build_cid_font(&self, doc: &mut Document, chars: &[char]) -> Result<ObjectId, String> {
        // 去重字符并按码点排序
        let mut uniq: Vec<char> = chars.to_vec();
        uniq.sort_unstable();
        uniq.dedup();
        let max_cid = uniq.last().map(|c| *c as u32).unwrap_or(0);

        // CIDToGIDMap 流：每个 CID（=Unicode 码点）2 字节大端 GID
        let mut cid2gid = vec![0u8; (max_cid as usize + 1) * 2];
        let subset_ttf = self.subset(&uniq)?;
        if let Ok(face) = ttf_parser::Face::parse(&subset_ttf, 0) {
            for &ch in &uniq {
                if let Some(gid) = face.glyph_index(ch) {
                    let off = (ch as u32 as usize) * 2;
                    cid2gid[off..off + 2].copy_from_slice(&gid.0.to_be_bytes());
                }
            }
        }
        let cid2gid_id = doc.add_object(Object::Stream(Self::compressed_stream(cid2gid)));

        // W 宽度数组：[[cid [w...]] ...]
        let mut w_entries: Vec<Object> = Vec::with_capacity(uniq.len());
        let mut face = ttf_parser::Face::parse(&self.ttf_data, 0).ok();
        for &ch in &uniq {
            let w = face
                .as_mut()
                .and_then(|f| f.glyph_index(ch).and_then(|g| f.glyph_hor_advance(g)))
                .unwrap_or(self.units_per_em / 2);
            let w1000 = (w as u32 * 1000 / self.units_per_em as u32) as i64;
            w_entries.push(Object::Array(vec![
                Object::Integer(ch as u32 as i64),
                Object::Array(vec![Object::Integer(w1000)]),
            ]));
        }

        // FontFile2：嵌入子集化后的 TTF（仅保留用到的字形）
        let font_file_id = doc.add_object(Object::Stream(Self::compressed_stream(subset_ttf)));

        // FontDescriptor
        let descriptor_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"FontDescriptor".to_vec())),
            (
                b"FontName".to_vec(),
                Object::Name(self.postscript_name.as_bytes().to_vec()),
            ),
            (b"Flags".to_vec(), Object::Integer(32)),
            (b"FontBBox".to_vec(), Object::Array(self.bbox.iter().map(|v| Object::Integer(*v as i64)).collect())),
            (b"ItalicAngle".to_vec(), Object::Integer(0)),
            (b"Ascent".to_vec(), Object::Integer(self.ascent as i64)),
            (b"Descent".to_vec(), Object::Integer(self.descent as i64)),
            (b"CapHeight".to_vec(), Object::Integer(self.cap_height as i64)),
            (b"StemV".to_vec(), Object::Integer(80)),
            (b"FontFile2".to_vec(), Object::Reference(font_file_id)),
        ])));

        // CIDFontType2
        let cid_font_id = doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"Font".to_vec())),
            (b"Subtype".to_vec(), Object::Name(b"CIDFontType2".to_vec())),
            (
                b"BaseFont".to_vec(),
                Object::Name(self.postscript_name.as_bytes().to_vec()),
            ),
            (
                b"CIDSystemInfo".to_vec(),
                Object::Dictionary(Dictionary::from_iter(vec![
                    (b"Registry".to_vec(), Object::string_literal("Adobe")),
                    (b"Ordering".to_vec(), Object::string_literal("Identity")),
                    (b"Supplement".to_vec(), Object::Integer(0)),
                ])),
            ),
            (b"FontDescriptor".to_vec(), Object::Reference(descriptor_id)),
            (b"CIDToGIDMap".to_vec(), Object::Reference(cid2gid_id)),
            (b"DW".to_vec(), Object::Integer(1000)),
            (b"W".to_vec(), Object::Array(w_entries)),
        ])));

        // ToUnicode CMap
        let to_unicode_id = doc.add_object(Object::Stream(Self::compressed_stream(
            build_to_unicode_cmap(&uniq).into_bytes(),
        )));

        // Type0 字体
        Ok(doc.add_object(Object::Dictionary(Dictionary::from_iter(vec![
            (b"Type".to_vec(), Object::Name(b"Font".to_vec())),
            (b"Subtype".to_vec(), Object::Name(b"Type0".to_vec())),
            (
                b"BaseFont".to_vec(),
                Object::Name(self.postscript_name.as_bytes().to_vec()),
            ),
            (b"Encoding".to_vec(), Object::Name(b"Identity-H".to_vec())),
            (
                b"DescendantFonts".to_vec(),
                Object::Array(vec![Object::Reference(cid_font_id)]),
            ),
            (b"ToUnicode".to_vec(), Object::Reference(to_unicode_id)),
        ]))))
    }

    /// 字形子集化：仅保留用到的字形（含复合字形递归依赖），重建独立 TTF 字节
    fn subset(&self, chars: &[char]) -> Result<Vec<u8>, String> {
        let ttf = &self.ttf_data;
        let tables = parse_tables(ttf, 0)?;
        let head = get_table(ttf, &tables, b"head").ok_or("字体缺少 head 表")?;
        let hhea = get_table(ttf, &tables, b"hhea").ok_or("字体缺少 hhea 表")?;
        let maxp = get_table(ttf, &tables, b"maxp").ok_or("字体缺少 maxp 表")?;
        let loca = get_table(ttf, &tables, b"loca").ok_or("字体缺少 loca 表")?;
        let glyf = get_table(ttf, &tables, b"glyf").ok_or("字体缺少 glyf 表")?;
        let hmtx = get_table(ttf, &tables, b"hmtx").ok_or("字体缺少 hmtx 表")?;

        let loca_fmt = u16::from_be_bytes([head[50], head[51]]);

        // 收集用到的字形（含复合组件），同时记录 码点→旧gid 用于重建 cmap
        let face = ttf_parser::Face::parse(ttf, 0).map_err(|e| format!("字体解析失败: {:?}", e))?;
        let mut glyphs: BTreeSet<u16> = BTreeSet::new();
        let mut char_glyphs: Vec<(u16, u16)> = Vec::new();
        glyphs.insert(0); // .notdef 必须保留
        for &ch in chars {
            if let Some(gid) = face.glyph_index(ch) {
                collect_component_glyphs(loca, loca_fmt, glyf, gid.0, &mut glyphs);
                let cp = ch as u32;
                if cp <= 0xFFFF {
                    char_glyphs.push((cp as u16, gid.0));
                }
            }
        }
        drop(face);

        // 新 gid 映射（按旧 gid 升序重编号，0 = .notdef）
        let new_ids: Vec<u16> = glyphs.iter().copied().collect();
        let old_to_new: HashMap<u16, u16> = new_ids
            .iter()
            .enumerate()
            .map(|(i, g)| (*g, i as u16))
            .collect();

        // glyf 子集：复合字形需重映射组件 gid
        let mut new_glyf = Vec::new();
        let mut new_loca: Vec<u32> = vec![0];
        for &old in &new_ids {
            let (start, end) = loca_range(loca, loca_fmt, old);
            let raw = &glyf[start..end];
            if raw.len() >= 10 {
                let num_contours = i16::from_be_bytes([raw[0], raw[1]]);
                if num_contours < 0 {
                    new_glyf.extend(remap_composite(raw, &old_to_new)?);
                } else {
                    new_glyf.extend_from_slice(raw);
                }
            }
            new_loca.push(new_glyf.len() as u32);
        }

        // hmtx 子集：numberOfHMetrics = 子集字形数（全部显式宽度）
        let num_hmetrics = u16::from_be_bytes([hhea[34], hhea[35]]) as usize;
        let mut new_hmtx = Vec::with_capacity(new_ids.len() * 4);
        for &old in &new_ids {
            let oi = old as usize;
            let (advance, lsb) = if oi < num_hmetrics {
                let a = u16::from_be_bytes([hmtx[oi * 4], hmtx[oi * 4 + 1]]);
                let l = i16::from_be_bytes([hmtx[oi * 4 + 2], hmtx[oi * 4 + 3]]);
                (a, l)
            } else {
                // 超出 numberOfHMetrics 的部分：advance 取最后一个，lsb 在附加数组中
                let a = u16::from_be_bytes([
                    hmtx[(num_hmetrics - 1) * 4],
                    hmtx[(num_hmetrics - 1) * 4 + 1],
                ]);
                let base = num_hmetrics * 4 + (oi - num_hmetrics) * 2;
                let l = i16::from_be_bytes([hmtx[base], hmtx[base + 1]]);
                (a, l)
            };
            new_hmtx.extend_from_slice(&advance.to_be_bytes());
            new_hmtx.extend_from_slice(&lsb.to_be_bytes());
        }

        // cmap format 4 重建（仅保留用到的字符映射）
        let cmap_mappings: Vec<(u16, u16)> = char_glyphs
            .iter()
            .filter_map(|&(cp, old)| old_to_new.get(&old).map(|&g| (cp, g)))
            .collect();
        let new_cmap = build_cmap4(&cmap_mappings);

        // head：统一使用长 loca（indexToLocFormat = 1）
        let mut new_head = head.to_vec();
        new_head[50..52].copy_from_slice(&1u16.to_be_bytes());
        // maxp：更新 numGlyphs
        let mut new_maxp = maxp.to_vec();
        new_maxp[4..6].copy_from_slice(&(new_ids.len() as u16).to_be_bytes());

        // 子集化后 loca 使用长格式
        let mut loca_bytes = Vec::with_capacity(new_loca.len() * 4);
        for v in &new_loca {
            loca_bytes.extend_from_slice(&v.to_be_bytes());
        }

let keep = |tag: &[u8]| get_table(ttf, &tables, tag).map(|t| (tag.to_vec(), t.to_vec()));
        let mut out_tables: Vec<(Vec<u8>, Vec<u8>)> = vec![
            (b"cmap".to_vec(), new_cmap),
            (b"glyf".to_vec(), new_glyf),
            (b"head".to_vec(), new_head),
            (b"hhea".to_vec(), hhea.to_vec()),
            (b"hmtx".to_vec(), new_hmtx),
            (b"loca".to_vec(), loca_bytes),
            (b"maxp".to_vec(), new_maxp),
        ];
        // 可选表：name / post / OS/2 原样保留
        for tag in [b"name", b"post", b"OS/2"] {
            if let Some(t) = keep(tag) {
                out_tables.push(t);
            }
        }
        // 标记：重写 head 的 checkSumAdjustment 后校验和不再一致，但阅读器不强制校验
        rebuild_sfnt(&out_tables)
    }

    /// 构造已压缩的流对象（设置 FlateDecode Filter 并更新 Length）
    fn compressed_stream(content: Vec<u8>) -> Stream {
        let mut stream = Stream::new(Dictionary::new(), content);
        stream.compress().ok();
        stream
    }
}

// ==================== sfnt 解析与重建 ====================

type TableMap = Vec<(Vec<u8>, usize, usize)>; // tag, offset(相对文件), length

/// 解析 sfnt 表目录（TTC 中 base 为 face 起始位置；单 TTF 为 0）
fn parse_tables(data: &[u8], base: usize) -> Result<TableMap, String> {
    if base + 12 > data.len() {
        return Err("sfnt 头不完整".into());
    }
    let num = u16::from_be_bytes([data[base + 4], data[base + 5]]) as usize;
    if base + 12 + num * 16 > data.len() {
        return Err("sfnt 表目录越界".into());
    }
    let mut tables = Vec::with_capacity(num);
    for i in 0..num {
        let r = base + 12 + i * 16;
        let tag = data[r..r + 4].to_vec();
        let off = u32::from_be_bytes([data[r + 8], data[r + 9], data[r + 10], data[r + 11]])
            as usize
            + base;
        let len = u32::from_be_bytes([data[r + 12], data[r + 13], data[r + 14], data[r + 15]])
            as usize;
        if off + len > data.len() {
            return Err(format!("表 {:?} 越界", tag));
        }
        tables.push((tag, off, len));
    }
    Ok(tables)
}

/// 按 tag 取出表数据（相对 ttf 文件偏移切片）
fn get_table<'a>(ttf: &'a [u8], tables: &TableMap, tag: &[u8]) -> Option<&'a [u8]> {
    tables
        .iter()
        .find(|(t, _, _)| t.as_slice() == tag)
        .map(|(_, o, l)| &ttf[*o..*o + *l])
}

/// 从 TTC 中提取 face 的独立 sfnt：按表目录把各表（偏移相对 face 基址）重新打包
fn rebuild_face(data: &[u8], base: usize) -> Result<Vec<u8>, String> {
    let tables = parse_tables(data, base)?;
    let mut out: Vec<(Vec<u8>, Vec<u8>)> = Vec::with_capacity(tables.len());
    for (tag, off, len) in tables {
        out.push((tag, data[off..off + len].to_vec()));
    }
    rebuild_sfnt(&out)
}

/// 把 (tag, data) 表集合按 tag 排序后重建为完整 sfnt 文件
fn rebuild_sfnt(tables: &[(Vec<u8>, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let mut sorted: Vec<&(Vec<u8>, Vec<u8>)> = tables.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let n = sorted.len();
    if n == 0 || n > 4095 {
        return Err("表数量异常".into());
    }
    let header_len = 12 + n * 16;
    let entry_selector = (n as u32).ilog2() as u16;
    let search_range = 16u16 << entry_selector;
    let range_shift = (n as u16).wrapping_mul(16).wrapping_sub(search_range);

    let mut out = Vec::new();
    out.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]); // sfntVersion 1.0
    out.extend_from_slice(&(n as u16).to_be_bytes());
    out.extend_from_slice(&search_range.to_be_bytes());
    out.extend_from_slice(&entry_selector.to_be_bytes());
    out.extend_from_slice(&range_shift.to_be_bytes());

    // 数据偏移：记录表先占位（checksum 填 0），数据紧随其后、4 字节对齐
    let mut offsets = Vec::with_capacity(n);
    let mut off = header_len;
    for (_, data) in &sorted {
        offsets.push(off);
        off += (data.len() + 3) & !3;
    }
    for (i, (tag, data)) in sorted.iter().enumerate() {
        out.extend_from_slice(tag);
        out.extend_from_slice(&[0, 0, 0, 0]); // checksum（不校验）
        out.extend_from_slice(&(offsets[i] as u32).to_be_bytes());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    }
    for (_, data) in &sorted {
        out.extend_from_slice(data);
        while out.len() % 4 != 0 {
            out.push(0);
        }
    }
    Ok(out)
}

// ==================== 字形收集与子集 ====================

/// 读取 loca 中 glyph 的字节区间
fn loca_range(loca: &[u8], fmt: u16, gid: u16) -> (usize, usize) {
    let read = |i: usize| -> usize {
        if fmt == 0 {
            u16::from_be_bytes([loca[i * 2], loca[i * 2 + 1]]) as usize * 2
        } else {
            u32::from_be_bytes([loca[i * 4], loca[i * 4 + 1], loca[i * 4 + 2], loca[i * 4 + 3]])
                as usize
        }
    };
    (read(gid as usize), read(gid as usize + 1))
}

/// 递归收集复合字形依赖的所有 glyph
fn collect_component_glyphs(
    loca: &[u8],
    loca_fmt: u16,
    glyf: &[u8],
    gid: u16,
    out: &mut BTreeSet<u16>,
) {
    if !out.insert(gid) {
        return;
    }
    let (start, end) = loca_range(loca, loca_fmt, gid);
    let raw = &glyf[start..end];
    if raw.len() < 10 {
        return;
    }
    let num_contours = i16::from_be_bytes([raw[0], raw[1]]);
    if num_contours >= 0 {
        return; // 简单字形
    }
    // 复合字形：解析组件序列（glyf 格式见 OpenType 规范）
    let mut pos = 10usize;
    loop {
        if pos + 4 > raw.len() {
            return;
        }
        let flags = u16::from_be_bytes([raw[pos], raw[pos + 1]]);
        let comp = u16::from_be_bytes([raw[pos + 2], raw[pos + 3]]);
        collect_component_glyphs(loca, loca_fmt, glyf, comp, out);
        pos += 4;
        // 参数：1 word 标志 → 4 字节，否则 2 字节
        pos += if flags & 0x0001 != 0 { 4 } else { 2 };
        // 变换矩阵
        if flags & 0x0008 != 0 {
            pos += 2;
        } else if flags & 0x0040 != 0 {
            pos += 4;
        } else if flags & 0x0080 != 0 {
            pos += 8;
        }
        // 指令长度（仅第一个组件可能出现）
        if flags & 0x0100 != 0 {
            if pos + 2 > raw.len() {
                return;
            }
            let ilen = u16::from_be_bytes([raw[pos], raw[pos + 1]]) as usize;
            pos += 2 + ilen;
        }
        if flags & 0x0200 == 0 {
            return; // MORE_COMPONENTS 未设置 → 组件序列结束
        }
    }
}

/// 复合字形拷贝并重映射组件 glyphIndex
fn remap_composite(raw: &[u8], old_to_new: &HashMap<u16, u16>) -> Result<Vec<u8>, String> {
    let mut out = raw.to_vec();
    let mut pos = 10usize;
    loop {
        if pos + 4 > raw.len() {
            return Err("复合字形结构异常".into());
        }
        let flags = u16::from_be_bytes([raw[pos], raw[pos + 1]]);
        let old = u16::from_be_bytes([raw[pos + 2], raw[pos + 3]]);
        let new = *old_to_new.get(&old).ok_or("复合字形引用缺失字形")?;
        out[pos + 2..pos + 4].copy_from_slice(&new.to_be_bytes());
        pos += 4;
        pos += if flags & 0x0001 != 0 { 4 } else { 2 };
        if flags & 0x0008 != 0 {
            pos += 2;
        } else if flags & 0x0040 != 0 {
            pos += 4;
        } else if flags & 0x0080 != 0 {
            pos += 8;
        }
        if flags & 0x0100 != 0 {
            if pos + 2 > raw.len() {
                return Err("复合字形结构异常".into());
            }
            let ilen = u16::from_be_bytes([raw[pos], raw[pos + 1]]) as usize;
            pos += 2 + ilen;
        }
        if flags & 0x0200 == 0 {
            break;
        }
    }
    Ok(out)
}

/// 重建 cmap format 4 子表：每个用到的字符一个 segment（idDelta = 新gid - 码点，u16 回绕）
fn build_cmap4(mappings: &[(u16, u16)]) -> Vec<u8> {
    // mappings: (码点, 新gid)；idDelta 按 16 位无符号回绕，解析时 (cp + delta) mod 65536 = gid
    let mut segs: Vec<(u16, u16)> = mappings
        .iter()
        .map(|&(cp, gid)| (cp, (gid as u32).wrapping_sub(cp as u32) as u16))
        .collect();
    segs.sort_by_key(|(s, _)| *s);
    // 结束 segment（0xFFFF）
    segs.push((0xFFFF, 1));

    let seg_count = segs.len() as u16;
    let seg_x2 = seg_count * 2;
    let entry_selector = (seg_count as u32).ilog2() as u16;
    let search_range = 2u16 << entry_selector;
    let range_shift = seg_x2.wrapping_sub(search_range);

    let mut out = Vec::new();
    // encoding record：platform 3 (Windows) / encoding 1 (Unicode BMP)
    out.extend_from_slice(&0u16.to_be_bytes()); // version
    out.extend_from_slice(&1u16.to_be_bytes()); // numTables
    out.extend_from_slice(&3u16.to_be_bytes()); // platformID
    out.extend_from_slice(&1u16.to_be_bytes()); // encodingID
    out.extend_from_slice(&12u32.to_be_bytes()); // subtable offset

    // subtable header
    out.extend_from_slice(&4u16.to_be_bytes()); // format
    let len_pos = out.len();
    out.extend_from_slice(&0u16.to_be_bytes()); // length（占位，稍后回填）
    out.extend_from_slice(&0u16.to_be_bytes()); // language
    out.extend_from_slice(&seg_x2.to_be_bytes());
    out.extend_from_slice(&search_range.to_be_bytes());
    out.extend_from_slice(&entry_selector.to_be_bytes());
    out.extend_from_slice(&range_shift.to_be_bytes());
    // endCode[]
    for (end, _) in &segs {
        out.extend_from_slice(&end.to_be_bytes());
    }
    out.extend_from_slice(&0u16.to_be_bytes()); // reservedPad
    // startCode[]
    for (start, _) in &segs {
        out.extend_from_slice(&start.to_be_bytes());
    }
    // idDelta[]
    for (_, delta) in &segs {
        out.extend_from_slice(&delta.to_be_bytes());
    }
    // idRangeOffset[]（全部 0，无 glyphIdArray）
    for _ in &segs {
        out.extend_from_slice(&0u16.to_be_bytes());
    }
    // 回填 length（subtable 长度，不含 cmap 表头 12 字节）
    let len = (out.len() - 12) as u16;
    out[len_pos..len_pos + 2].copy_from_slice(&len.to_be_bytes());
    out
}

/// 生成 ToUnicode CMap（CID = Unicode 码点）
fn build_to_unicode_cmap(chars: &[char]) -> String {
    let mut cmap = String::from(
        "/CIDInit /ProcSet findresource begin\n\
         12 dict begin\n\
         begincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n\
         /CMapType 2 def\n\
         1 begincodespacerange\n\
         <0000> <FFFF>\n\
         endcodespacerange\n",
    );
    cmap.push_str(&format!("{} beginbfchar\n", chars.len()));
    for &ch in chars {
        let cp = ch as u32;
        // 非 BMP 字符（如 emoji）用 8 位十六进制
        let hex = if cp > 0xFFFF {
            format!("{:08X}", cp)
        } else {
            format!("{:04X}", cp)
        };
        cmap.push_str(&format!("<{}> <{}>\n", hex, hex));
    }
    cmap.push_str("endbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    cmap
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_font_subset() {
        let mut font = match load_system_font() {
            Ok(f) => f,
            Err(e) => {
                eprintln!("跳过（无系统字体）: {}", e);
                return;
            }
        };
        // 子集化前后字体可解析，字符映射保留
        let chars: Vec<char> = "DocMorph 中文水印测试".chars().collect();
        let subset = font.subset(&chars).unwrap();
        let face = ttf_parser::Face::parse(&subset, 0).expect("子集字体可解析");
        for &ch in &chars {
            assert!(
                face.glyph_index(ch).is_some(),
                "子集应保留字符映射: {}",
                ch
            );
        }
        // 子集体积应远小于完整字体（完整中文字体 >1MB，子集 <200KB）
        assert!(
            subset.len() < 200 * 1024,
            "子集应小于 200KB，实际 {}",
            subset.len()
        );
        // 宽度查询正常
        assert!(font.advance('中') > 0);
        println!("✅ 字体子集化通过：{} 字节", subset.len());
    }
}
