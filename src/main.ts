import { createApp } from "vue";
import './styles.css'
import App from "./App.vue";
import Login from './components/Login.vue'
import Arena from './components/Arena.vue'
import * as VueRouter from 'vue-router'
import PrimeVue from 'primevue/config'
import ConfirmationService from 'primevue/confirmationservice'
import ToastService from 'primevue/toastservice';
import 'primevue/resources/themes/aura-light-blue/theme.css'
import 'primeicons/primeicons.css'
import 'material-symbols'
import 'primeflex/primeflex.css'

const app = createApp(App)
const routes = [
    { path: '/', component: Login },
    { path: '/arena/:name', component: Arena, props: true }
]
const router = VueRouter.createRouter({
    history: VueRouter.createWebHashHistory(),
    routes,
})
app.use(router)
app.use(PrimeVue)
app.use(ConfirmationService)
app.use(ToastService)
app.mount("#app");
