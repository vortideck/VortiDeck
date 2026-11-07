<template>
  <div class="preview-file-widget w-full">
    <!-- Loading State -->
    <div v-if="loading" class="file-placeholder">
      <div class="icon">⏳</div>
      <p class="file-type">Loading preview...</p>
    </div>

    <!-- Error State -->
    <div v-else-if="error" class="file-placeholder error">
      <div class="icon">⚠️</div>
      <p class="file-type">Error</p>
      <p class="file-name">{{ error }}</p>
    </div>

    <!-- Image Preview -->
    <div v-else-if="previewType === 'image' && imageData" class="image-preview">
      <img
        :src="imageData"
        :alt="fileName"
        class="w-full h-auto rounded-xl object-contain bg-themeBackground border border-themeBorder"
        style="max-height: 300px;"
        @error="handleImageError"
      />
    </div>

    <!-- Video Preview -->
    <div v-else-if="previewType === 'video' && videoSrc" class="video-preview">
      <video
        :src="videoSrc"
        controls
        class="w-full rounded-xl bg-themeBackground border border-themeBorder"
        style="max-height: 300px;"
        @error="handleVideoError"
      >
        Your browser does not support video playback.
      </video>
    </div>

    <!-- Audio Preview -->
    <div v-else-if="previewType === 'audio' && audioSrc" class="audio-preview">
      <div class="icon">🎵</div>
      <p class="file-name">{{ fileName }}</p>
      <audio
        :src="audioSrc"
        controls
        class="w-full mt-2"
        @error="handleAudioError"
      >
        Your browser does not support audio playback.
      </audio>
    </div>

    <!-- Text/Code Preview -->
    <div v-else-if="previewType === 'text' && textContent" class="text-preview">
      <pre class="preview-code"><code>{{ truncatedText }}</code></pre>
      <p v-if="isTruncated" class="truncated-notice">... (truncated, showing first 1000 chars)</p>
    </div>

    <!-- PDF/Other File Placeholder -->
    <div v-else class="file-placeholder">
      <div class="icon">{{ getFileIcon() }}</div>
      <p class="file-type">{{ getFileTypeName() }}</p>
      <p class="file-name">{{ fileName }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { readTextFile } from '@tauri-apps/plugin-fs'
import { convertFileSrc } from '@tauri-apps/api/core'

interface Props {
  filePath: string
  fileName: string
  fileType: string  // MIME type from backend
}

const props = withDefaults(defineProps<Props>(), {
  filePath: '',
  fileName: '',
  fileType: 'application/octet-stream'
})

const loading = ref(false)
const error = ref<string | null>(null)
const textContent = ref<string | null>(null)

// Determine preview type from MIME type
const previewType = computed(() => {
  const mime = props.fileType.toLowerCase()
  if (mime.startsWith('image/')) return 'image'
  if (mime.startsWith('video/')) return 'video'
  if (mime.startsWith('audio/')) return 'audio'
  if (mime.startsWith('text/') || mime === 'application/json' || mime === 'application/xml') return 'text'
  if (mime === 'application/pdf') return 'pdf'
  return 'file'
})

// Use convertFileSrc for all binary files (images, video, audio)
const imageData = computed(() => props.filePath ? convertFileSrc(props.filePath) : null)
const videoSrc = computed(() => props.filePath ? convertFileSrc(props.filePath) : null)
const audioSrc = computed(() => props.filePath ? convertFileSrc(props.filePath) : null)

const truncatedText = computed(() => {
  if (!textContent.value) return ''
  const maxLength = 1000
  return textContent.value.length > maxLength
    ? textContent.value.substring(0, maxLength)
    : textContent.value
})

const isTruncated = computed(() => {
  return textContent.value ? textContent.value.length > 1000 : false
})

const getFileIcon = (): string => {
  switch (previewType.value) {
    case 'pdf': return '📄'
    case 'audio': return '🎵'
    case 'video': return '🎬'
    case 'image': return '🖼️'
    default: return '📎'
  }
}

const getFileTypeName = (): string => {
  switch (previewType.value) {
    case 'pdf': return 'PDF Document'
    case 'audio': return 'Audio File'
    case 'video': return 'Video File'
    case 'image': return 'Image File'
    default: props.fileType || 'File'
  }
}

// Load preview (only for text files, binary files use convertFileSrc)
const loadPreview = async () => {
  if (!props.filePath) return

  loading.value = true
  error.value = null
  textContent.value = null

  try {
    const type = previewType.value

    if (type === 'text') {
      // Read text files as text
      textContent.value = await readTextFile(props.filePath)
    }
    // Images, videos, and audio use convertFileSrc directly (no loading needed)
  } catch (e: any) {
    console.error('Preview error:', e)

    // Detect permission/access errors
    const errorMsg = e.message || String(e)
    if (errorMsg.includes('permission') || errorMsg.includes('denied') || errorMsg.includes('scope')) {
      error.value = `Permission denied. File must be in workspace (Documents/VortiDeck/workspace/)`
    } else if (errorMsg.includes('not found') || errorMsg.includes('ENOENT')) {
      error.value = `File not found: ${props.fileName}`
    } else {
      error.value = errorMsg || 'Failed to load file preview'
    }
  } finally {
    loading.value = false
  }
}

// Handle media loading errors (permission denied, file not found, etc.)
const handleImageError = (event: Event) => {
  console.error('Image load error:', event)
  error.value = `Failed to load image. File may be outside workspace (Documents/VortiDeck/workspace/) or permissions denied.`
}

const handleVideoError = (event: Event) => {
  console.error('Video load error:', event)
  error.value = `Failed to load video. File may be outside workspace (Documents/VortiDeck/workspace/) or permissions denied.`
}

const handleAudioError = (event: Event) => {
  console.error('Audio load error:', event)
  error.value = `Failed to load audio. File may be outside workspace (Documents/VortiDeck/workspace/) or permissions denied.`
}

// Load preview when props change
watch(() => [props.filePath, previewType.value], loadPreview, { immediate: true })
</script>

<style scoped>
.preview-file-widget {
  padding: 0.5rem;
}

.image-preview {
  width: 100%;
}

.text-preview {
  width: 100%;
  background: hsl(var(--background3));
  border: 1px solid hsl(var(--border));
  border-radius: 0.5rem;
  padding: 0.75rem;
  max-height: 200px;
  overflow: auto;
}

.preview-code {
  margin: 0;
  padding: 0;
  font-family: 'Monaco', 'Menlo', 'Consolas', monospace;
  font-size: 11px;
  color: hsl(var(--text));
  white-space: pre-wrap;
  word-break: break-all;
}

.truncated-notice {
  margin-top: 0.5rem;
  font-size: 11px;
  color: hsl(var(--text3));
  font-style: italic;
  text-align: center;
}

.file-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
  background: hsl(var(--background));
  border-radius: 0.75rem;
  border: 2px dashed hsl(var(--border));
}

.file-placeholder.error {
  border-color: #ef4444;
  background: rgba(239, 68, 68, 0.1);
}

.audio-preview {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 1rem;
  background: hsl(var(--background));
  border: 1px solid hsl(var(--border));
  border-radius: 0.75rem;
}

.video-preview {
  width: 100%;
}

.icon {
  font-size: 2.5rem;
  margin-bottom: 0.5rem;
}

.file-type {
  margin: 0.25rem 0;
  font-size: 13px;
  font-weight: 500;
  color: hsl(var(--text));
}

.file-name {
  margin: 0.25rem 0;
  font-size: 11px;
  color: hsl(var(--text3));
  word-break: break-all;
  text-align: center;
  max-width: 200px;
}
</style>
