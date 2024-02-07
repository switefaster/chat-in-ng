<script setup lang="ts">
import InputText from 'primevue/inputtext';
import InlineMessage from 'primevue/inlinemessage';
import Textarea from 'primevue/textarea';
import Button from 'primevue/button';
import MessagePanel from './MessagePanel.vue'
import Toast from 'primevue/toast';
import Chip from 'primevue/chip';
import Fieldset from 'primevue/fieldset';
import ConfirmPopup from 'primevue/confirmpopup';
import Sidebar from 'primevue/sidebar';
import Dialog from 'primevue/dialog';
import { useConfirm } from 'primevue/useconfirm';
import { useToast } from 'primevue/usetoast'
import { ref, Ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event';

type GameState = 'Joining' | 'Assigning' | 'Fighting';
type PlayerState = 'Alive' | 'Out';
type ClientState = 'Spectator' | { Player: [PlayerState, string?] };
type ReadyButtonMessage = '准备' | '取消准备' | undefined;
type VoteState = {
    [name: string]: 'agreed' | 'objection'
};

const props = defineProps<{
    name: string,
}>()
const toast = useToast()
const message = ref('')
const sending = ref(false)
const messages: Ref<Array<{
    name: string,
    ng: string | undefined,
    content: string,
}>> = ref([])
const gameState: Ref<GameState> = ref('Joining')
const clients: Ref<{ [name: string]: ClientState }> = ref({})
const assignmentVisible = ref(false)
const endingVisible = ref(false)
const endingReason = ref('')
const votingVisible = ref(false)
const whoToAssign = ref('')
const readyPlayer: Ref<Set<string>> = ref(new Set())
const timerTime = ref(600)
let timerHandle = -1
const readyButtonMessage: Ref<ReadyButtonMessage> = ref(undefined)
const readyButtonIcon = ref('pi pi-check')
const readyButtonLoading = ref(false)
const voteState: Ref<VoteState> = ref({})
const voting: Ref<'free' | 'voting' | 'voted'> = ref('free')

document.addEventListener('contextmenu', (event) => event.preventDefault())
document.addEventListener('keydown', (event) => {
    if (event.code === 'F5') {
        event.preventDefault()
    }
})

const toMessageRole = (name: string) => {
    if (name.length === 0) {
        return 'system'
    }
    const client = clients.value[name]
    if (client === undefined) {
        return 'spectator'
    }
    if (client === 'Spectator') {
        return 'spectator'
    } else {
        const [state, _] = (client as { Player: [PlayerState, string?] }).Player
        if (state === 'Alive') {
            return 'player_alive'
        } else {
            return 'player_out'
        }
    }
}

const getNgWord = (client: ClientState) => {
    if (client === undefined || client === 'Spectator') {
        return undefined
    } else {
        const [_, word] = (client as { Player: [PlayerState, string?] }).Player
        return word
    }
}

const spectators = computed(() => {
    const result = []
    for (const name in clients.value) {
        if (clients.value[name] === 'Spectator') {
            result.push(name)
        }
    }
    return result
})

const sortedPlayers = computed(() => {
    const result: Array<[string, PlayerState]> = []
    for (const name in clients.value) {
        if (clients.value[name] !== 'Spectator') {
            const [state, _] = (clients.value[name] as { Player: [PlayerState, string?] }).Player
            result.push([name, state])
        }
    }
    return result.sort(([nameA, stateA], [nameB, stateB]) => {
        if (stateA === 'Out' && stateB === 'Alive') {
            return 1
        } else if (stateB === 'Out' && stateA === 'Alive') {
            return -1
        } else {
            return nameA.localeCompare(nameB)
        }
    })
})

const playerCount = computed(() => {
    let count = 0
    for (const name in clients.value) {
        if (clients.value[name] !== 'Spectator') {
            count++
        }
    }
    return count
})

const undeterminedPlayers = computed(() => {
    const result = []
    for (const name in clients.value) {
        if (clients.value[name] !== 'Spectator') {
            if (voteState.value[name] === undefined) {
                result.push(name)
            }
        }
    }
    return result
})

const agreedPlayers = computed(() => {
    const result = []
    for (const name in clients.value) {
        if (clients.value[name] !== 'Spectator') {
            if (voteState.value[name] !== undefined && voteState.value[name] === 'agreed') {
                result.push(name)
            }
        }
    }
    return result
})

const objectionPlayers = computed(() => {
    const result = []
    for (const name in clients.value) {
        if (clients.value[name] !== 'Spectator') {
            if (voteState.value[name] !== undefined && voteState.value[name] === 'objection') {
                result.push(name)
            }
        }
    }
    return result
})

const alivePlayerCount = computed(() => {
    return readyPlayer.value.size
})

listen('received_message', (event) => {
    const [sender, content] = event.payload as [string, string]
    messages.value.push({
        name: sender,
        ng: getNgWord(clients.value[sender]),
        content: content,
    })
});

listen('overview', (event) => {
    const [newClients, newGameState] = event.payload as [{ [name: string]: ClientState }, GameState]
    gameState.value = newGameState;
    clients.value = newClients;
    if (newGameState === 'Joining') {
        readyButtonMessage.value = '准备'
        readyButtonIcon.value = 'pi pi-check'
    }
})

listen('message_history', (event) => {
    const history = event.payload as Array<[string, string]>
    messages.value = history.map(([sender, content], _, __) => {
        return {
            name: sender,
            ng: getNgWord(clients.value[sender]),
            content: content,
        }
    })
})

listen('player_join', (event) => {
    const player = event.payload as string
    switch (gameState.value) {
        case 'Joining':
            clients.value[player] = {
                Player: ['Alive', undefined]
            }
            break
        case 'Assigning':
        case 'Fighting':
            clients.value[player] = 'Spectator'
            break
    }
    toast.add({
        severity: 'info',
        summary: '玩家加入',
        detail: `${player} 加入了游戏！`,
        life: 3000,
    })
})

listen('player_quit', (event) => {
    const player = event.payload as string
    delete clients.value[player]
    readyPlayer.value.delete(player)
    toast.add({
        severity: 'info',
        summary: '玩家退出',
        detail: `${player} 退出了游戏！`,
        life: 3000,
    })
})

listen('game_start', (event) => {
    const words = event.payload as {
        [name: string]: string
    }
    gameState.value = 'Fighting'
    readyButtonMessage.value = undefined
    for (const player in words) {
        if (clients.value[player] !== 'Spectator') {
            (clients.value[player] as { Player: [PlayerState, string?] }).Player[1] = words[player]
        }
    }
})

listen('assign_start', (event) => {
    gameState.value = 'Assigning'
    whoToAssign.value = event.payload as string
    assignmentVisible.value = true
    readyPlayer.value.clear()
    readyButtonMessage.value = '准备'
    readyButtonIcon.value = 'pi pi-check'
})

listen('player_out', (event) => {
    const [quitter, word, suicide] = event.payload as [string, string, boolean]
    if (!suicide) {
        toast.add({
            severity: 'warn',
            summary: '玩家出局',
            detail: `${quitter} 因说出 ${word} 出局了！`,
            life: 3000,
        })
    } else {
        toast.add({
            severity: 'warn',
            summary: '玩家出局',
            detail: `${quitter} 自爆出局了！`,
            life: 3000,
        })
    }
    if (clients.value[quitter] !== 'Spectator') {
        (clients.value[quitter] as { Player: [PlayerState, string?] }).Player[0] = 'Out'
    }
    readyPlayer.value.delete(quitter)
})

listen('player_ready', (event) => {
    const player = event.payload as string
    readyPlayer.value.add(player)
})

listen('player_not_ready', (event) => {
    const player = event.payload as string
    readyPlayer.value.delete(player)
})

listen('timer_reset', (event) => {
    const timer = event.payload as number
    timerTime.value = timer
    clearTimeout(timerHandle)
    timerHandle = setInterval(() => {
        if (timerTime.value > 0) {
            timerTime.value--
        }
    }, 1000)
})

listen('start_vote_abort', (_) => {
    assignmentVisible.value = false
    votingVisible.value = true
})

listen('voted_abort', (event) => {
    const [voter, agree] = event.payload as [string, boolean]
    if (agree) {
        voteState.value[voter] = 'agreed'
    } else {
        voteState.value[voter] = 'objection'
    }
    if (voter === props.name) {
        voting.value = 'voted'
    }
})

listen('vote_abort_result', (event) => {
    const abort = event.payload as boolean
    votingVisible.value = false
    if (abort) {
        assignmentVisible.value = false
        endingReason.value = `中止投票通过，游戏重新开始`
        endingVisible.value = true
        cleanupForNextRound()
    } else {
        votingVisible.value = false
        voteState.value = {}
        voting.value = 'free'
    }
})

const cleanupForNextRound = () => {
    messages.value = []
    gameState.value = 'Joining'
    for (const player in clients.value) {
        clients.value[player] = {
            Player: ['Alive', undefined]
        }
    }
    whoToAssign.value = ''
    readyPlayer.value.clear()
    clearTimeout(timerHandle)
    assignmentLoading.value = false
    assignmentError.value = ''
    assignmentWord.value = ''
    readyButtonIcon.value = 'pi pi-check'
    readyButtonLoading.value = false
    readyButtonMessage.value = '准备'
    voteState.value = {}
    votingVisible.value = false
    voting.value = 'free'
}

listen('game_win', (event) => {
    const [winner, word] = event.payload as [string, string]
    votingVisible.value = false
    assignmentVisible.value = false
    endingReason.value = `${winner} 存活到了最后，TA自始至终都没有说出 ${word}！`
    endingVisible.value = true
    cleanupForNextRound()
})

listen('timeout', (_) => {
    votingVisible.value = false
    assignmentVisible.value = false
    endingReason.value = `由于超时，游戏自动重新开始`
    endingVisible.value = true
    cleanupForNextRound()
})

listen('unproceedable', (_) => {
    votingVisible.value = false
    assignmentVisible.value = false
    endingReason.value = `游戏无法继续进行，自动重新开始`
    endingVisible.value = true
    cleanupForNextRound()
})

const sendMessage = () => {
    if (message.value.length === 0) {
        return;
    }
    sending.value = true
    invoke('send_message', {
        message: message.value
    }).then(() => {
        message.value = ""
    }).catch((err) => {
        toast.add({
            severity: 'error',
            summary: '发送失败',
            detail: err,
            life: 3000,
        })
    }).finally(() => {
        sending.value = false
    })
}

const confirm = useConfirm()
const confirmSurrender = (event: MouseEvent) => {
    confirm.require({
        target: event.currentTarget as HTMLElement | undefined,
        message: '真的要自爆吗？',
        icon: 'pi pi-exclamation-triangle',
        rejectClass: 'p-button-secondary p-button-outlined p-button-sm',
        acceptClass: 'p-button-warning p-button-sm',
        rejectLabel: '算了',
        acceptLabel: '投了',
        accept: () => {
            invoke('suicide').catch((err) => {
                toast.add({
                    severity: 'error',
                    summary: '自爆发送失败',
                    detail: err,
                    life: 3000,
                })
            })
        }
    })
}

const confirmAbort = (event: MouseEvent) => {
    confirm.require({
        target: event.currentTarget as HTMLElement | undefined,
        message: '真的要发起重开投票吗？',
        icon: 'pi pi-exclamation-triangle',
        rejectClass: 'p-button-secondary p-button-outlined p-button-sm',
        acceptClass: 'p-button-danger p-button-sm',
        rejectLabel: '算了',
        acceptLabel: '玩不下去一点',
        accept: () => {
            voting.value = 'voted'
            invoke('request_abort').catch((err) => {
                toast.add({
                    severity: 'error',
                    summary: '申请发送失败',
                    detail: err,
                    life: 3000,
                })
            })
        }
    })
}

const assignmentLoading = ref(false)
const assignmentWord = ref('')
const assignmentError = ref('')

const assignWord = () => {
    assignmentError.value = ''
    assignmentLoading.value = true
    invoke('assign_word', {
        word: assignmentWord.value
    }).catch((err) => {
        assignmentLoading.value = false
        assignmentError.value = err
    })
}

listen('assign_result', (event) => {
    const excuse = event.payload as string
    if (excuse === null) {
        assignmentVisible.value = false
        assignmentWord.value = ''
        assignmentError.value = ''
        assignmentLoading.value = false
    } else {
        assignmentLoading.value = false
        assignmentError.value = excuse
    }
})

const readyCancelHandler = () => {
    if (readyButtonMessage.value === '准备') {
        readyButtonLoading.value = true
        invoke('set_ready')
            .catch((err) => {
                toast.add({
                    severity: 'error',
                    summary: '准备失败',
                    detail: err,
                    life: 3000,
                })
            })
    } else if (readyButtonMessage.value === '取消准备') {
        readyButtonLoading.value = true
        invoke('cancel_ready')
            .catch((err) => {
                toast.add({
                    severity: 'error',
                    summary: '取消准备失败',
                    detail: err,
                    life: 3000,
                })
            })
    }
}

listen('ready_result', (event) => {
    if (readyButtonMessage.value === '准备') {
        const excuse = event.payload as string | undefined
        if (excuse === null) {
            readyButtonMessage.value = '取消准备'
            readyButtonIcon.value = 'pi pi-times'
            readyButtonLoading.value = false
        } else {
            readyButtonLoading.value = false
            toast.add({
                severity: 'error',
                summary: '准备失败',
                detail: excuse,
                life: 3000,
            })
        }
    } else if (readyButtonMessage.value === '取消准备') {
        readyButtonMessage.value = '准备'
        readyButtonIcon.value = 'pi pi-check'
        readyButtonLoading.value = false
    }
})

const vote = (agree: boolean) => {
    voting.value = 'voting'
    invoke('vote_abort', {
        abort: agree
    }).catch((err) => {
        toast.add({
            severity: 'error',
            summary: '投票失败',
            detail: err,
            life: 3000,
        })
        voting.value = 'free'
    })
}

const enterPressedOnTextArea = (event: KeyboardEvent) => {
    if (event.code === 'Enter' && !event.shiftKey) {
        sendMessage()
        event.preventDefault()
    }
}

const sidebarVisible = ref(false)

invoke('flush_response_queue')
listen('server_event', (_) => {
    invoke('flush_response_queue')
})
</script>

<template>
    <div class="w-full arena-upper">
        <!--Top Navigation-->
        <div class="sticky top-0 w-full p-1 bg-cyan-200 grid">
            <Toast></Toast>
            <ConfirmPopup></ConfirmPopup>
            <div class="col-2">
                <Button severity="warning" @click="confirmSurrender($event)" text rounded>
                    <span class="material-symbols-outlined">
                        explosion
                    </span>
                </Button>
                <Button severity="danger" @click="confirmAbort($event)" text rounded>
                    <span class="material-symbols-outlined">
                        chat_error
                    </span>
                </Button>
            </div>
            <div class="col flex flex-row align-content-center justify-content-center gap-1">
                <Chip label="玩家加入中" icon="pi pi-sign-in" v-if="gameState === 'Joining'"></Chip>
                <Chip label="分配词语中" icon="pi pi-user-edit" v-if="gameState === 'Assigning'"></Chip>
                <Chip label="游戏进行中" icon="pi pi-comments" v-if="gameState === 'Fighting'"></Chip>
                <Button :label="alivePlayerCount.toString() + '/' + playerCount.toString()" icon="pi pi-user" text
                    rounded></Button>
                <Button v-if="gameState === 'Assigning'" label="指定词语"
                    @click="assignmentVisible = !votingVisible && true"></Button>
                <Button v-if="readyButtonMessage !== undefined" :label="readyButtonMessage" severity="success"
                    :icon="readyButtonIcon" :loading="readyButtonLoading" @click="readyCancelHandler"></Button>
                <Button v-if="gameState === 'Fighting'" :label="Math.floor(timerTime / 60) + ':' + timerTime % 60"
                    icon="pi pi-clock" text rounded></Button>
            </div>
            <div class="col-1">
                <Button style="float: right;" @click="sidebarVisible = true" text rounded>
                    <span class="material-symbols-outlined">
                        group
                    </span>
                </Button>
            </div>
            <Sidebar header="在线用户" v-model:visible="sidebarVisible" position="right">
                <p class="text-xl">玩家({{ sortedPlayers.length }})</p>
                <p class="text-base" v-for="player in sortedPlayers"
                    :class="{ 'p-text-secondary': player[1] === 'Out', 'text-orange-500': player[1] === 'Alive' }">{{
                        player[0] }}</p>
                <p class="text-xl">观战({{ spectators.length }})</p>
                <p class="text-base" v-for="spec in spectators">{{ spec }}</p>
            </Sidebar>
        </div>
        <!--Messages-->
        <Dialog v-model:visible="assignmentVisible" modal header="指定词语" :style="{ width: '25rem' }">
            <span class="p-text-secondary block mb-5">请为 <strong>{{ whoToAssign }}</strong> 指定NG词</span>
            <div class="flex flex-column p-fluid gap-2">
                <InlineMessage v-show="assignmentError !== ''">{{ assignmentError }}</InlineMessage>
                <InputText class="flex-auto" autocomplete="off" placeholder="NG词" v-model="assignmentWord" />
                <Button type="button" class="mt-2" label="发送" @click="assignWord" :loading="assignmentLoading"></Button>
            </div>
        </Dialog>
        <Dialog v-model:visible="votingVisible" modal header="中止投票" :closable=false :close-on-escape=false
            :style="{ 'max-height': '30rem', width: '40rem' }">
            <div class="grid h-full">
                <div class="col flex flex-column p-fluid justify-content-center">
                    <Fieldset class="overflow-auto" legend="反对" style="height: 100%;">
                        <p class="text-base" v-for="obj in objectionPlayers">{{ obj }}</p>
                    </Fieldset>
                    <Button type="button" class="mt-2" label="反对" severity="danger" style="height: 3rem;"
                        :loading="voting === 'voting'" :disabled="voting === 'voted'" @click="vote(false)"></Button>
                </div>
                <Fieldset class="col overflow-auto" legend="未投票">
                    <p class="text-base" v-for="und in undeterminedPlayers">{{ und }}</p>
                </Fieldset>
                <div class="col flex flex-column p-fluid justify-content-center">
                    <Fieldset class="overflow-auto" legend="同意">
                        <p class="text-base" v-for="agree in agreedPlayers">{{ agree }}</p>
                    </Fieldset>
                    <Button type="button" class="mt-2" label="同意" severity="success" style="height: 3rem;"
                        :loading="voting === 'voting'" :disabled="voting === 'voted'" @click="vote(true)"></Button>
                </div>
            </div>
        </Dialog>
        <Dialog v-model:visible="endingVisible" modal header="游戏结束" :style="{ width: '25rem' }">
            <p class="text-base">{{ endingReason }}</p>
        </Dialog>
        <div class="flex flex-column w-full h-full gap-1 p-1 message-scroll overflow-auto">
            <MessagePanel v-for="msg in messages" :name="msg.name" :ng="msg.ng" :message="msg.content"
                :role="toMessageRole(msg.name)">
            </MessagePanel>
            <div class="message-anchor"></div>
        </div>
    </div>
    <!--Input Area-->
    <div class="card fixed bottom-0 left-0 w-full flex p-fluid gap-2 p-1 bg-white">
        <Textarea class="w-full" rows="3" placeholder="发射言弹" v-model="message" @keypress="enterPressedOnTextArea($event)"
            style="resize: none;"></Textarea>
        <Button icon="pi pi-send" @click="sendMessage" :loading="sending" style="width: 6rem;"></Button>
    </div>
</template>

<style scoped>
.arena-upper {
    padding-bottom: 84px;
}

.message-scroll * {
    overflow-anchor: none;
}

.message-anchor {
    overflow-anchor: auto;
    height: 1px;
}
</style>
