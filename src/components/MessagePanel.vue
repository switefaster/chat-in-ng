<script setup lang="ts">
import Panel from 'primevue/panel';
import Badge from 'primevue/badge';

const props = defineProps<{
    name: string,
    role: 'player_alive' | 'player_out' | 'spectator' | 'system',
    ng: string | undefined,
    message: string,
}>()
</script>

<template>
    <Panel>
        <template #header>
            <div class="flex flex-row gap-2 align-content-center">
                <span class="font-bold" v-if="props.role !== 'system'">{{ props.name }}</span>
                <span class="font-bold" v-else>系统</span>
                <Badge value="玩家" severity="success" v-if="props.role === 'player_alive'"></Badge>
                <Badge value="玩家" severity="danger" v-if="props.role === 'player_out'"></Badge>
                <Badge value="观战" severity="info" v-if="props.role === 'spectator'"></Badge>
                <Badge value="系统" severity="warning" v-if="props.role === 'system'"></Badge>
                <span class="p-secondary underline" style="align-items: center;"
                    v-if="props.ng !== undefined && props.ng !== null">{{
                        props.ng }}</span>
            </div>
        </template>
        <p class="m-0" :class="{ 'text-orange-500': props.role === 'system' }" style="word-wrap: break-word;">
            {{ props.message }}
        </p>
    </Panel>
</template>

<style scoped></style>
