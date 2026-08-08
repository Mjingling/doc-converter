<template>
  <n-modal v-model:show="show" preset="card" :title="t('donate.title')" style="width: 360px">
    <div class="donate">
      <p class="donate-desc">{{ t("donate.desc") }}</p>
      <NTabs type="segment" animated size="small" justify-content="center">
        <NTabPane name="wechat" :tab="t('donate.wechat')">
          <div class="qr-wrap">
            <img class="qr" src="/donate/wechat.png" :alt="t('donate.wechat')" />
          </div>
        </NTabPane>
        <NTabPane name="alipay" :tab="t('donate.alipay')">
          <div class="qr-wrap">
            <img class="qr" src="/donate/alipay.png" :alt="t('donate.alipay')" />
          </div>
        </NTabPane>
      </NTabs>
      <p class="donate-tip">{{ t("donate.tip") }}</p>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
import { NModal, NTabs, NTabPane } from "naive-ui";
import { useI18n } from "vue-i18n";

const { t } = useI18n();
const show = defineModel<boolean>("show", { default: false });
</script>

<style scoped>
.donate-desc {
  margin: 0 0 12px;
  font-size: 13px;
  color: var(--text-sub);
  text-align: center;
}
.qr-wrap {
  display: flex;
  justify-content: center;
  padding: 10px 0 6px;
}
.qr {
  width: 210px;
  height: 210px;
  border-radius: 10px;
}
.donate-tip {
  margin: 6px 0 0;
  font-size: 12px;
  color: var(--text-muted);
  text-align: center;
}
</style>
