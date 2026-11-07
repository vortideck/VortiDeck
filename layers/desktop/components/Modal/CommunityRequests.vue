<template lang="pug">
BaseModal(:is-open="isOpen" @close="$emit('close')")
  div(class="vd-modal-card relative w-full max-w-[50rem] max-h-[85vh] flex flex-col")
        //- Header
        div(class="flex items-center justify-between p-6 border-b border-themeBorder")
          div(class="flex items-center gap-x-3")
            //- Back button when in submission view
            button(v-if="currentView === 'submission'" @click="currentView = 'list'" class="text-themeText/60 hover:text-themeText transition-colors p-1")
              svg(class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7")
            div
              h3(class="text-2xl font-semibold text-themeText") {{ currentView === 'list' ? 'Community Feature Requests' : 'Submit Your Idea' }}
              p(class="text-sm text-themeText/60 mt-1") {{ currentView === 'list' ? 'Vote on features and submit your ideas' : 'Help shape the future of VortiDeck' }}
          div(class="flex items-center gap-x-3")
            //- Stats badges with better contrast (only show in list view)
            div(v-if="currentView === 'list'" class="flex gap-x-2")
              span(class="px-2 py-1 bg-themeBackground3 text-themeText rounded-full text-xs font-medium border border-themeBorder") {{ totalRequests }} requests
              span(class="px-2 py-1 bg-themeBackground3 text-green-500 rounded-full text-xs font-medium border border-green-500/30") {{ inDevelopmentCount }} building
              span(class="px-2 py-1 bg-themeBackground3 text-blue-500 rounded-full text-xs font-medium border border-blue-500/30") {{ completedCount }} completed
            button(@click="$emit('close')" class="vd-modal-close-btn")
              svg(class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

        //- Tab Navigation (only in list view)
        div(v-if="currentView === 'list'" class="flex border-b border-themeBorder px-6")
          button(
            v-for="tab in tabs"
            :key="tab.id"
            @click="activeTab = tab.id"
            :class="[activeTab === tab.id ? 'border-blue-500 text-blue-500' : 'border-transparent text-themeText3 hover:text-themeText']"
            class="flex items-center gap-x-2 px-4 py-3 border-b-2 transition-all duration-200"
          )
            component(:is="tab.icon" class="w-4 h-4")
            span(class="text-sm font-medium") {{ tab.label }}
            span(v-if="tab.count" class="ml-1 px-1.5 py-0.5 bg-themeBackground2 rounded-full text-xs") {{ tab.count }}

        //- Body Content
        div(v-if="currentView === 'list'" class="flex-1 overflow-y-auto p-6")
          //- In Development Tab
          div(v-show="activeTab === 'development'" class="space-y-4")
            div(v-for="item in inDevelopment" :key="item.id" class="bg-themeBackground2 rounded-lg p-4 border border-themeBorder")
              div(class="flex items-start justify-between mb-3")
                div
                  h4(class="font-semibold text-lg flex items-center gap-x-2")
                    span {{ item.title }}
                    span(class="px-2 py-0.5 bg-green-500/20 text-green-400 rounded-full text-xs") In Progress
                  p(class="text-sm text-themeText3 mt-1") {{ item.description }}
                span(class="text-sm text-blue-400 font-medium") {{ item.progress }}% Complete
              
              //- Progress bar
              div(class="relative h-2 bg-themeBackground3 rounded-full overflow-hidden")
                div(
                  :style="`width: ${item.progress}%`"
                  class="absolute top-0 left-0 h-full bg-gradient-to-r from-blue-500 to-purple-500 transition-all duration-500"
                )
              
              div(class="flex items-center justify-between mt-3")
                p(class="text-xs text-themeText3") ETA: {{ item.eta }}
                div(class="flex items-center gap-x-2")
                  Fa6SolidHeart(class="w-3 h-3 text-red-400")
                  span(class="text-xs text-themeText3") {{ item.votes }} votes

          //- Popular Tab
          div(v-show="activeTab === 'popular'" class="space-y-3")
            div(
              v-for="item in popularRequests"
              :key="item.id"
              class="bg-themeBackground2 rounded-lg p-4 border border-themeBorder hover:border-blue-500/30 transition-all duration-200"
            )
              div(class="flex items-start justify-between")
                div(class="flex-1")
                  h4(class="font-semibold") {{ item.title }}
                  p(class="text-sm text-themeText3 mt-1") {{ item.description }}
                  div(class="flex items-center gap-x-2 mt-2")
                    span(
                      v-for="tag in item.tags"
                      :key="tag"
                      class="px-2 py-0.5 bg-themeBackground3 text-xs rounded-full text-themeText3"
                    ) #{{ tag }}
                
                div(class="flex flex-col items-center ml-4")
                  button(
                    @click="voteForRequest(item.id)"
                    :class="[hasVoted(item.id) ? 'bg-red-500/20 text-red-400' : 'bg-themeBackground3 text-themeText3 hover:bg-red-500/20 hover:text-red-400']"
                    class="p-3 rounded-lg transition-all duration-200"
                  )
                    Fa6SolidHeart(class="w-5 h-5")
                  span(class="text-sm font-semibold mt-1") {{ item.votes }}

          //- Recent Tab
          div(v-show="activeTab === 'recent'" class="space-y-3")
            div(
              v-for="item in recentRequests"
              :key="item.id"
              class="bg-themeBackground2 rounded-lg p-4 border border-themeBorder"
            )
              div(class="flex items-start justify-between")
                div
                  div(class="flex items-center gap-x-2 mb-2")
                    span(class="px-2 py-0.5 bg-themeBackground3 text-blue-400 border border-blue-400/30 rounded-full text-xs") New
                    span(class="text-xs text-themeText3") {{ formatDate(item.createdAt) }}
                  h4(class="font-semibold") {{ item.title }}
                  p(class="text-sm text-themeText3 mt-1") {{ item.description }}
                
                button(
                  @click="voteForRequest(item.id)"
                  class="flex flex-col items-center"
                )
                  div(class="p-2 bg-themeBackground3 rounded-lg hover:bg-red-500/20 transition-colors")
                    Fa6SolidHeart(class="w-4 h-4 text-themeText3 hover:text-red-400")
                  span(class="text-xs mt-1") {{ item.votes }}

          //- Completed Tab
          div(v-show="activeTab === 'completed'" class="space-y-3")
            div(
              v-for="item in completedRequests"
              :key="item.id"
              class="bg-themeBackground2 rounded-lg p-4 border border-green-500/20"
            )
              div(class="flex items-start justify-between")
                div
                  div(class="flex items-center gap-x-2 mb-2")
                    Fa6SolidCheck(class="w-4 h-4 text-green-400")
                    h4(class="font-semibold") {{ item.title }}
                  p(class="text-sm text-themeText3") {{ item.description }}
                  p(class="text-xs text-green-400 mt-2") Completed {{ formatDate(item.completedAt) }}
                
                div(class="text-center")
                  Fa6SolidHeart(class="w-4 h-4 text-red-400 mx-auto")
                  span(class="text-xs text-themeText3") {{ item.votes }}

        //- Submission Form View
        div(v-if="currentView === 'submission'" class="flex-1 overflow-y-auto p-6")
          div(class="space-y-4 max-w-2xl mx-auto")
            //- Title Input
            div
              label(for="idea-title" class="block text-sm font-medium text-themeText mb-2") 
                span Title 
                span(class="text-red-400") *
              input#idea-title(
                v-model="formData.title"
                type="text"
                placeholder="Give your idea a clear title..."
                maxlength="100"
                class="w-full bg-themeBackground2 rounded-lg px-4 py-2 text-sm placeholder-themeText/40 border border-themeBorder focus:border-blue-500/50 focus:outline-none"
              )
              p(class="text-xs text-themeText/40 mt-1") {{ formData.title.length }}/100 characters

            //- Category Selection
            div
              label(for="idea-category" class="block text-sm font-medium text-themeText mb-2") 
                span Category 
                span(class="text-red-400") *
              select#idea-category(
                v-model="formData.category"
                class="w-full bg-themeBackground2 rounded-lg px-4 py-2 text-sm border border-themeBorder focus:border-blue-500/50 focus:outline-none"
              )
                option(value="" disabled) Select a category
                option(value="feature") 🚀 New Feature
                option(value="integration") 🔗 Integration
                option(value="ui") 🎨 UI/UX Improvement
                option(value="performance") ⚡ Performance
                option(value="bug") 🐛 Bug Fix
                option(value="other") 📦 Other

            //- Description Textarea
            div
              label(for="idea-description" class="block text-sm font-medium text-themeText mb-2") 
                span Description 
                span(class="text-red-400") *
              textarea#idea-description(
                v-model="formData.description"
                rows="6"
                placeholder="Describe your idea in detail. What problem does it solve? How would it work?"
                maxlength="500"
                class="w-full bg-themeBackground2 rounded-lg px-4 py-2 text-sm placeholder-themeText/40 border border-themeBorder focus:border-blue-500/50 focus:outline-none resize-none"
              )
              p(class="text-xs text-themeText/40 mt-1") {{ formData.description.length }}/500 characters

            //- Tags Input
            div
              label(class="block text-sm font-medium text-themeText mb-2") Tags (Optional)
              div(class="flex flex-wrap gap-2 mb-2")
                span(
                  v-for="tag in formData.tags"
                  :key="tag"
                  class="px-3 py-1 bg-themeBackground3 text-themeText text-xs rounded-full flex items-center gap-x-1 border border-themeBorder"
                )
                  span #{{ tag }}
                  button(@click="removeTag(tag)" class="ml-1 text-themeText/60 hover:text-red-400")
                    svg(class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                      path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")
              input(
                v-model="tagInput"
                @keyup.enter="addTag"
                type="text"
                placeholder="Add tags (press Enter)"
                class="w-full bg-themeBackground2 rounded-lg px-4 py-2 text-sm placeholder-themeText/40 border border-themeBorder focus:border-blue-500/50 focus:outline-none"
              )
              p(class="text-xs text-themeText/40 mt-1") Add relevant tags to help categorize your idea

            //- Priority Selection
            div
              label(class="block text-sm font-medium text-themeText mb-2") Priority Level
              div(class="flex gap-2")
                button(
                  v-for="priority in priorities"
                  :key="priority.value"
                  @click="formData.priority = priority.value"
                  :class="[formData.priority === priority.value ? 'border-blue-500 bg-blue-500/10' : 'border-themeBorder']"
                  class="flex-1 px-3 py-2 rounded-lg border text-sm transition-all"
                )
                  span(:class="priority.color") {{ priority.icon }} {{ priority.label }}

        //- Footer
        div(v-if="currentView === 'list'" class="p-6 border-t border-themeBorder bg-themeBackground2/50")
          div(class="flex items-center justify-between")
            p(class="text-sm text-themeText/60") Have an idea? Help shape VortiDeck's future
            button(
              @click="currentView = 'submission'"
              class="px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white rounded-lg text-sm font-medium transition-colors flex items-center gap-x-2"
            ) 
              Fa6SolidLightbulb(class="w-4 h-4")
              span Submit Your Idea
              
        //- Submission Footer
        div(v-if="currentView === 'submission'" class="p-6 border-t border-themeBorder bg-themeBackground2/50")
          div(class="flex items-center justify-between")
            p(class="text-xs text-themeText/40") All submissions are reviewed by our team
            div(class="flex gap-3")
              button(
                @click="cancelSubmission"
                class="px-4 py-2 text-themeText/60 hover:text-themeText transition-colors"
              ) Cancel
              button(
                @click="submitIdea"
                :disabled="!isFormValid"
                class="px-4 py-2 bg-blue-500 hover:bg-blue-600 disabled:bg-gray-600 disabled:cursor-not-allowed text-white rounded-lg text-sm font-medium transition-colors"
              ) Submit Idea
</template>

<script setup lang="ts">
import BaseModal from "./BaseModal.vue";
import { ref, computed, watch } from 'vue'
import { load } from '@tauri-apps/plugin-store'
import { useMainStore } from '@/stores/Main'
import Fa6SolidFire from '~icons/fa6-solid/fire?width=16px&height=16px'
import Fa6SolidRocket from '~icons/fa6-solid/rocket?width=16px&height=16px'
import Fa6SolidClock from '~icons/fa6-solid/clock?width=16px&height=16px'
import Fa6SolidCheck from '~icons/fa6-solid/check?width=16px&height=16px'
import Fa6SolidHeart from '~icons/fa6-solid/heart?width=16px&height=16px'
import Fa6SolidLightbulb from '~icons/fa6-solid/lightbulb?width=16px&height=16px'

const props = defineProps<{
  isOpen: boolean
  startInSubmission?: boolean
}>()

const emit = defineEmits(['close'])
const mainStore = useMainStore()

// State
const currentView = ref<'list' | 'submission'>('list')
const activeTab = ref('development')
const votedRequests = ref<string[]>([])
let voteStore: any = null

// Form data for submission
const formData = ref({
  title: '',
  category: '',
  description: '',
  tags: [] as string[],
  priority: 'normal',
  email: ''
})

const tagInput = ref('')

const priorities = [
  { value: 'low', label: 'Low', icon: '🔵', color: 'text-blue-400' },
  { value: 'normal', label: 'Normal', icon: '🟢', color: 'text-green-400' },
  { value: 'high', label: 'High', icon: '🟡', color: 'text-yellow-400' },
  { value: 'critical', label: 'Critical', icon: '🔴', color: 'text-red-400' }
]

// Mock data
const inDevelopment = ref([
  {
    id: 'dev1',
    title: 'Discord Rich Presence',
    description: 'Show what you\'re doing in VortiDeck on Discord',
    progress: 45,
    eta: '2 weeks',
    votes: 324
  },
  {
    id: 'dev2',
    title: 'Linux Support',
    description: 'Native Linux build with full feature parity',
    progress: 20,
    eta: '1 month',
    votes: 512
  },
  {
    id: 'dev3',
    title: 'Custom Themes',
    description: 'Create and share custom visual themes',
    progress: 5,
    eta: '6 weeks',
    votes: 234
  }
])

const popularRequests = ref([
  {
    id: 'pop1',
    title: 'Spotify Integration',
    description: 'Control Spotify playback directly from your deck',
    votes: 156,
    tags: ['integration', 'music']
  },
  {
    id: 'pop2',
    title: 'Auto Backup',
    description: 'Automatic cloud backup for all deck configurations',
    votes: 98,
    tags: ['feature', 'cloud']
  },
  {
    id: 'pop3',
    title: 'Plugin Marketplace',
    description: 'Browse and install community plugins',
    votes: 87,
    tags: ['feature', 'community']
  },
  {
    id: 'pop4',
    title: 'Voice Commands',
    description: 'Trigger actions with voice recognition',
    votes: 76,
    tags: ['feature', 'accessibility']
  }
])

const recentRequests = ref([
  {
    id: 'rec1',
    title: 'Macro Recording',
    description: 'Record complex action sequences',
    votes: 12,
    createdAt: new Date(Date.now() - 1000 * 60 * 60 * 2) // 2 hours ago
  },
  {
    id: 'rec2',
    title: 'OBS WebSocket v5',
    description: 'Support for the latest OBS WebSocket protocol',
    votes: 8,
    createdAt: new Date(Date.now() - 1000 * 60 * 60 * 24) // 1 day ago
  },
  {
    id: 'rec3',
    title: 'Touch Bar Support',
    description: 'MacBook Touch Bar as additional deck',
    votes: 3,
    createdAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 2) // 2 days ago
  }
])

const completedRequests = ref([
  {
    id: 'comp1',
    title: 'Dark Mode',
    description: 'System-wide dark theme support',
    votes: 432,
    completedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 7) // 1 week ago
  },
  {
    id: 'comp2',
    title: 'Twitch Integration',
    description: 'Stream controls and chat commands',
    votes: 298,
    completedAt: new Date(Date.now() - 1000 * 60 * 60 * 24 * 14) // 2 weeks ago
  }
])

// Tabs configuration
const tabs = computed(() => [
  { id: 'development', label: 'In Development', icon: Fa6SolidRocket, count: inDevelopment.value.length },
  { id: 'popular', label: 'Popular', icon: Fa6SolidFire, count: null },
  { id: 'recent', label: 'Recent', icon: Fa6SolidClock, count: recentRequests.value.length },
  { id: 'completed', label: 'Completed', icon: Fa6SolidCheck, count: completedRequests.value.length }
])

// Computed
const totalRequests = computed(() => 
  inDevelopment.value.length + popularRequests.value.length + recentRequests.value.length + completedRequests.value.length
)

const inDevelopmentCount = computed(() => inDevelopment.value.length)
const completedCount = computed(() => completedRequests.value.length)

const isFormValid = computed(() => {
  return formData.value.title.trim() !== '' &&
         formData.value.category !== '' &&
         formData.value.description.trim() !== ''
})

// Methods
const hasVoted = (requestId: string) => {
  return votedRequests.value.includes(requestId)
}

const voteForRequest = async (requestId: string) => {
  const index = votedRequests.value.indexOf(requestId)
  
  if (index > -1) {
    // Remove vote
    votedRequests.value.splice(index, 1)
    updateVoteCount(requestId, -1)
  } else {
    // Add vote
    votedRequests.value.push(requestId)
    updateVoteCount(requestId, 1)
  }
  
  // Save to store
  if (voteStore) {
    await voteStore.set('votedRequests', votedRequests.value)
    await voteStore.save()
  }
}

const updateVoteCount = (requestId: string, delta: number) => {
  // Update vote count in the appropriate array
  const allRequests = [
    ...popularRequests.value,
    ...recentRequests.value,
    ...inDevelopment.value
  ]
  
  const request = allRequests.find(r => r.id === requestId)
  if (request) {
    request.votes += delta
  }
}

// Tag management
const addTag = () => {
  const tag = tagInput.value.trim().toLowerCase()
  if (tag && !formData.value.tags.includes(tag) && formData.value.tags.length < 5) {
    formData.value.tags.push(tag)
    tagInput.value = ''
  }
}

const removeTag = (tag: string) => {
  const index = formData.value.tags.indexOf(tag)
  if (index > -1) {
    formData.value.tags.splice(index, 1)
  }
}

// Submission methods
const cancelSubmission = () => {
  // Reset form and go back to list
  formData.value = {
    title: '',
    category: '',
    description: '',
    tags: [],
    priority: 'normal',
    email: ''
  }
  tagInput.value = ''
  currentView.value = 'list'
}

const submitIdea = () => {
  if (!isFormValid.value) return

  // Add to recent requests
  recentRequests.value.unshift({
    id: `rec${Date.now()}`,
    title: formData.value.title,
    description: formData.value.description,
    votes: 0,
    createdAt: new Date()
  })

  // Show success notification
  mainStore.addNotification({
    title: 'Idea Submitted!',
    message: 'Thank you for your contribution. We\'ll review it soon.',
    type: 'success',
    autoClose: true
  })

  // Reset form and switch to recent tab
  cancelSubmission()
  activeTab.value = 'recent'
  
  console.log('Idea submitted:', formData.value)
}

const formatDate = (date: Date) => {
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  const hours = Math.floor(diff / (1000 * 60 * 60))
  const days = Math.floor(hours / 24)
  
  if (hours < 1) return 'Just now'
  if (hours < 24) return `${hours} hour${hours > 1 ? 's' : ''} ago`
  if (days < 7) return `${days} day${days > 1 ? 's' : ''} ago`
  if (days < 30) return `${Math.floor(days / 7)} week${Math.floor(days / 7) > 1 ? 's' : ''} ago`
  return date.toLocaleDateString()
}

// Watch for startInSubmission prop
watch(() => props.startInSubmission, (newValue) => {
  if (newValue && props.isOpen) {
    currentView.value = 'submission'
  }
})

// Watch for modal open/close
watch(() => props.isOpen, (newValue) => {
  if (newValue && props.startInSubmission) {
    currentView.value = 'submission'
  } else if (newValue === false) {
    // Reset view when modal closes
    currentView.value = 'list'
  }
})

// Load voted requests on mount
onMounted(async () => {
  try {
    voteStore = await load('community-votes.json', { autoSave: false })
    const saved = await voteStore.get('votedRequests')
    if (saved) {
      votedRequests.value = saved
    }
  } catch (error) {
    console.error('Failed to load vote data:', error)
  }
})
</script>

<style lang="scss">
@use "./modalTheme.scss" as *;

.bg-clip-text {
  -webkit-background-clip: text;
  background-clip: text;
}
</style>
