#!/usr/bin/env python3
"""生成捐赠模块占位收款码图片（微信/支付宝各一张）。
占位图仅用于占位，用户需将真实收款码替换到 public/donate/ 目录。"""
import os
from PIL import Image, ImageDraw, ImageFont

OUT_DIR = "/Users/yjz1/Desktop/workspace/doc-converter/public/donate"
S = 320  # 画布尺寸


def load_font(size: int):
    """优先加载系统中文字体，失败则用默认字体"""
    for path in [
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/System/Library/Fonts/Supplemental/Songti.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/System/Library/Fonts/Helvetica.ttc",
    ]:
        try:
            return ImageFont.truetype(path, size)
        except Exception:
            continue
    return ImageFont.load_default()


def draw_placeholder(path: str, brand: str, brand_en: str, color: tuple):
    """绘制占位图：品牌色边框 + 三个定位角 + 中央提示文字"""
    img = Image.new("RGB", (S, S), "white")
    d = ImageDraw.Draw(img)
    # 品牌色边框
    d.rectangle([0, 0, S - 1, S - 1], outline=color, width=3)
    # 三个二维码定位角
    corner = [(22, 22), (S - 22 - 70, 22), (22, S - 22 - 70)]
    for x, y in corner:
        d.rectangle([x, y, x + 70, y + 70], outline=color, width=8)
        d.rectangle([x + 24, y + 24, x + 46, y + 46], fill=color)
    # 中央提示文字
    try:
        font_title = ImageFont.truetype("/System/Library/Fonts/Hiragino Sans GB.ttc", 32)
        font_sub = ImageFont.truetype("/System/Library/Fonts/Hiragino Sans GB.ttc", 18)
        title, sub = brand, "请替换为你的收款码图片"
    except Exception:
        font_title, font_sub = ImageFont.load_default(), ImageFont.load_default()
        title, sub = brand_en, "Replace with your QR code"
    bbox = d.textbbox((0, 0), title, font=font_title)
    d.text(((S - (bbox[2] - bbox[0])) / 2, 140), title, font=font_title, fill=color)
    bbox2 = d.textbbox((0, 0), sub, font=font_sub)
    d.text(((S - (bbox2[2] - bbox2[0])) / 2, 196), sub, font=font_sub, fill="#999999")
    img.save(path)
    print(f"生成: {path}")


if __name__ == "__main__":
    os.makedirs(OUT_DIR, exist_ok=True)
    # 微信绿 #07C160 / 支付宝蓝 #1677FF
    draw_placeholder(os.path.join(OUT_DIR, "wechat.png"), "微信收款码", "WeChat Pay", (7, 193, 96))
    draw_placeholder(os.path.join(OUT_DIR, "alipay.png"), "支付宝收款码", "Alipay", (22, 119, 255))
