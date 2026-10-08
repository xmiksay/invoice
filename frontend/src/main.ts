import { createApp } from "vue";
import { createPinia } from "pinia";
import { createWebHistory } from "vue-router";
import App from "./App.vue";
import { createAppRouter } from "./router";
import { i18n } from "./i18n";
import { setUnauthorizedHandler } from "./api/client";
import "./style.css";

const router = createAppRouter(createWebHistory());

setUnauthorizedHandler(() => {
  const current = router.currentRoute.value;
  if (current.name === "login") return;
  void router.push({ name: "login", query: { redirect: current.fullPath } });
});

document.documentElement.lang = i18n.global.locale.value;

createApp(App).use(createPinia()).use(router).use(i18n).mount("#app");
