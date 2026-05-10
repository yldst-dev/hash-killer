import { createApp } from 'vue'
import App from './App.vue'
import './styles.css'
import { installWebviewLockdown } from './lib/lockdown'

installWebviewLockdown()
createApp(App).mount('#app')
