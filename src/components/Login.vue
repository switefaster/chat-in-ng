<script setup lang="ts">
import { Ref, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useRouter } from 'vue-router'
import Button from 'primevue/button';
import InputText from 'primevue/inputtext';
import Message from 'primevue/message';
import InlineMessage from 'primevue/inlinemessage';
import FocusTrap from 'primevue/focustrap';
const vFocustrap = FocusTrap;

const name = ref("")
const nameMessage = ref("昵称不能为空")
const loading = ref(false)
const errors: Ref<Array<{
    content: string,
    id: number
}>> = ref([])
let count = ref(0)

const inputCheck = () => {
    if (name.value.trim().length === 0) {
        nameMessage.value = "昵称不能为空"
        return
    }
    nameMessage.value = ""
}

const router = useRouter()
const login = () => {
    loading.value = true
    invoke("login", {
        name: name.value
    })
        .then((_res) => {
            router.push(`/arena/${name.value}`)
        })
        .catch((reason) => {
            errors.value.push({
                content: reason,
                id: count.value++
            })
        })
        .finally(() => loading.value = false)
}
</script>

<template>
    <div class="card flex flex-column justify-content-center align-content-center flex-wrap p-fluid w-full h-full">
        <p class="text-6xl font-bold">你说你🐎呢</p>
        <div v-focustrap class="w-full" style="max-width: 20rem;">
            <transition-group name="p-message" tag="div">
                <Message v-for="err of errors" :key="err.id" severity="error">{{ err.content }}</Message>
            </transition-group>
            <InlineMessage v-show="nameMessage !== ''">{{ nameMessage }}</InlineMessage>
            <div class="field flex flex-wrap align-items-center gap-2">
                <div class="p-input-icon-right">
                    <i class="pi pi-user"></i>
                    <InputText id="input" v-model="name" type="text" placeholder="昵称" @input="inputCheck" autofocus>
                    </InputText>
                </div>
            </div>
            <Button type="button" label="加入" class="mt-2" icon="pi pi-sign-in" icon-pos="right" :loading="loading"
                @click="login"></Button>
        </div>
    </div>
</template>

<style scoped></style>
