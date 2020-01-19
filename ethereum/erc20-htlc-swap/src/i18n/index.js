import VueI18n from 'vue-i18n';
import { getLanguage } from '@/util/common';

/* eslint-disable */
Vue.use(VueI18n)
// 语言国际化
const borwserLang = getLanguage() || 'zh'
export default new VueI18n({
  locale: borwserLang,
  messages: {
    'zh': require('./langs/zh'),
    'en': require('./langs/en')
  }
})
