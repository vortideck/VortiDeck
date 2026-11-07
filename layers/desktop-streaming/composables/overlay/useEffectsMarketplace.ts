import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface MarketplaceEffect {
  id: string;
  name: string;
  description: string;
  type: 'emote_rain' | 'confetti' | 'fireworks' | 'particle_system' | 'custom_animation';
  category: 'celebration' | 'alert' | 'seasonal' | 'premium' | 'custom';
  price: number;
  currency: 'coins' | 'usd';
  creator: {
    id: string;
    name: string;
    verified: boolean;
  };
  preview: {
    thumbnail: string;
    video?: string;
    gif?: string;
  };
  config: EffectMarketplaceConfig;
  assets: EffectAssets;
  requirements: {
    min_version: string;
    features: string[];
  };
  metadata: {
    downloads: number;
    rating: number;
    reviews: number;
    created_at: string;
    updated_at: string;
    tags: string[];
  };
  ownership: {
    owned: boolean;
    purchased_at?: string;
    license_type: 'personal' | 'commercial' | 'unlimited';
  };
}

export interface EffectMarketplaceConfig {
  // Base properties that ALL effects have
  intensity_range: [number, number];
  duration_range: [number, number];
  customizable_properties: string[];
  
  // Type-specific configurations
  emote_rain?: {
    default_emotes: string[];
    custom_emotes_supported: boolean;
    max_emotes: number;
    rain_patterns: string[];
  };
  
  particle_system?: {
    particle_types: string[];
    physics_enabled: boolean;
    collision_detection: boolean;
    max_particles: number;
  };
  
  animation?: {
    keyframes: any[];
    easing_functions: string[];
    loop_support: boolean;
  };
}

export interface EffectAssets {
  // Files that come with the effect
  images?: string[];        // URLs to image assets
  sounds?: string[];        // URLs to sound assets  
  animations?: string[];    // URLs to animation files
  shaders?: string[];       // URLs to custom shaders
  fonts?: string[];         // URLs to custom fonts
  emote_packs?: {          // Bundled emote packs
    id: string;
    name: string;
    emotes: string[];
  }[];
}

export interface UserEffectInstance {
  effect_id: string;
  user_config: any;
  custom_name?: string;
  enabled: boolean;
  triggers: string[];      // ['follow', 'subscribe', 'donation', 'manual']
  last_used?: string;
}

export const useEffectsMarketplace = () => {
  // User's owned effects
  const ownedEffects = ref<MarketplaceEffect[]>([]);
  const userEffectInstances = ref<UserEffectInstance[]>([]);
  
  // Marketplace browse
  const marketplaceEffects = ref<MarketplaceEffect[]>([]);
  const categories = ref(['celebration', 'alert', 'seasonal', 'premium', 'custom']);
  const loading = ref(false);
  const error = ref<string | null>(null);
  
  // Filters
  const searchQuery = ref('');
  const selectedCategory = ref<string | null>(null);
  const priceRange = ref([0, 100]);
  const sortBy = ref<'popular' | 'newest' | 'price_low' | 'price_high' | 'rating'>('popular');

  // Computed
  const filteredEffects = computed(() => {
    let filtered = marketplaceEffects.value;
    
    // Search filter
    if (searchQuery.value) {
      const query = searchQuery.value.toLowerCase();
      filtered = filtered.filter(effect => 
        effect.name.toLowerCase().includes(query) ||
        effect.description.toLowerCase().includes(query) ||
        effect.metadata.tags.some(tag => tag.toLowerCase().includes(query))
      );
    }
    
    // Category filter
    if (selectedCategory.value) {
      filtered = filtered.filter(effect => effect.category === selectedCategory.value);
    }
    
    // Price filter
    filtered = filtered.filter(effect => 
      effect.price >= priceRange.value[0] && effect.price <= priceRange.value[1]
    );
    
    // Sort
    filtered.sort((a, b) => {
      switch (sortBy.value) {
        case 'newest':
          return new Date(b.metadata.created_at).getTime() - new Date(a.metadata.created_at).getTime();
        case 'price_low':
          return a.price - b.price;
        case 'price_high':
          return b.price - a.price;
        case 'rating':
          return b.metadata.rating - a.metadata.rating;
        case 'popular':
        default:
          return b.metadata.downloads - a.metadata.downloads;
      }
    });
    
    return filtered;
  });

  const ownedEffectIds = computed(() => 
    ownedEffects.value.map(effect => effect.id)
  );

  // API Integration
  const fetchMarketplaceEffects = async (page = 1, limit = 20) => {
    loading.value = true;
    error.value = null;
    
    try {
      // Call your API through Tauri
      const response = await invoke<{effects: MarketplaceEffect[], total: number}>('fetch_marketplace_effects', {
        page,
        limit,
        category: selectedCategory.value,
        search: searchQuery.value,
        sort: sortBy.value
      });
      
      marketplaceEffects.value = response.effects;
      return response;
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      console.error('Failed to fetch marketplace effects:', err);
      return { effects: [], total: 0 };
    } finally {
      loading.value = false;
    }
  };

  const fetchOwnedEffects = async () => {
    try {
      const effects = await invoke<MarketplaceEffect[]>('get_user_owned_effects');
      ownedEffects.value = effects;
      
      const instances = await invoke<UserEffectInstance[]>('get_user_effect_instances');
      userEffectInstances.value = instances;
      
      return effects;
    } catch (err) {
      console.error('Failed to fetch owned effects:', err);
      return [];
    }
  };

  // Purchase effect
  const purchaseEffect = async (effectId: string, paymentMethod: 'coins' | 'card') => {
    loading.value = true;
    error.value = null;
    
    try {
      const result = await invoke<{success: boolean, transaction_id: string}>('purchase_effect', {
        effect_id: effectId,
        payment_method: paymentMethod
      });
      
      if (result.success) {
        // Refresh owned effects
        await fetchOwnedEffects();
        
        // Create default instance
        await createEffectInstance(effectId);
      }
      
      return result;
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      console.error('Failed to purchase effect:', err);
      return { success: false, transaction_id: '' };
    } finally {
      loading.value = false;
    }
  };

  // Create effect instance with user config
  const createEffectInstance = async (effectId: string, customConfig?: any) => {
    const effect = ownedEffects.value.find(e => e.id === effectId);
    if (!effect) throw new Error('Effect not owned');
    
    const instance: UserEffectInstance = {
      effect_id: effectId,
      user_config: customConfig || getDefaultConfig(effect),
      enabled: true,
      triggers: ['manual'], // Start with manual trigger
    };
    
    const result = await invoke<UserEffectInstance>('create_effect_instance', { instance });
    userEffectInstances.value.push(result);
    
    return result;
  };

  // Get default config for an effect
  const getDefaultConfig = (effect: MarketplaceEffect) => {
    const config = effect.config;
    
    switch (effect.type) {
      case 'emote_rain':
        return {
          emotes: config.emote_rain?.default_emotes || ['poggers', 'kappa'],
          intensity: Math.min(...config.intensity_range),
          duration: Math.min(...config.duration_range),
          speed: 50,
          count: 25
        };
      
      case 'confetti':
        return {
          intensity: Math.min(...config.intensity_range),
          duration: Math.min(...config.duration_range),
          colors: ['#ff0000', '#00ff00', '#0000ff'],
          particle_count: 50
        };
      
      default:
        return {};
    }
  };

  // Use/trigger an effect
  const triggerMarketplaceEffect = async (instanceId: string, triggerData?: any) => {
    const instance = userEffectInstances.value.find(i => 
      i.effect_id === instanceId || userEffectInstances.value.indexOf(i).toString() === instanceId
    );
    
    if (!instance || !instance.enabled) {
      throw new Error('Effect instance not found or disabled');
    }
    
    const effect = ownedEffects.value.find(e => e.id === instance.effect_id);
    if (!effect) {
      throw new Error('Effect not found');
    }
    
    // Download assets if needed
    await downloadEffectAssets(effect);
    
    // Build effect config with user customizations
    const effectConfig = {
      id: `marketplace_${instance.effect_id}_${Date.now()}`,
      type: effect.type,
      action: 'start',
      timestamp: Date.now(),
      config: {
        ...instance.user_config,
        ...triggerData,
        // Include asset URLs
        assets: effect.assets
      }
    };
    
    // Send through existing dynamic effects system
    return await invoke("send_overlay_message", {
      message: JSON.stringify({
        type: "DYNAMIC_EFFECT",
        effect: effectConfig,
        timestamp: Date.now(),
      }),
    });
  };

  // Download and cache effect assets
  const downloadEffectAssets = async (effect: MarketplaceEffect) => {
    try {
      // Check if assets are already cached
      const cached = await invoke<boolean>('check_effect_assets_cached', { 
        effect_id: effect.id 
      });
      
      if (!cached) {
        // Download assets through Tauri
        await invoke('download_effect_assets', {
          effect_id: effect.id,
          assets: effect.assets
        });
      }
      
      return true;
    } catch (err) {
      console.error('Failed to download effect assets:', err);
      return false;
    }
  };

  // Preview effect (free preview)
  const previewEffect = async (effectId: string) => {
    const effect = marketplaceEffects.value.find(e => e.id === effectId);
    if (!effect) throw new Error('Effect not found');
    
    // Use preview config (limited version)
    const previewConfig = {
      id: `preview_${effectId}`,
      type: effect.type,
      action: 'start',
      timestamp: Date.now(),
      config: {
        ...getDefaultConfig(effect),
        duration: 3000, // Always 3 seconds for preview
        intensity: 30,  // Lower intensity
        preview_mode: true
      }
    };
    
    return await invoke("send_overlay_message", {
      message: JSON.stringify({
        type: "DYNAMIC_EFFECT",
        effect: previewConfig,
        timestamp: Date.now(),
      }),
    });
  };

  // Effect management
  const updateEffectInstance = async (instanceId: string, updates: Partial<UserEffectInstance>) => {
    const index = userEffectInstances.value.findIndex(i => i.effect_id === instanceId);
    if (index === -1) throw new Error('Instance not found');
    
    userEffectInstances.value[index] = { ...userEffectInstances.value[index], ...updates };
    
    return await invoke('update_effect_instance', {
      instance_id: instanceId,
      updates
    });
  };

  const deleteEffectInstance = async (instanceId: string) => {
    const index = userEffectInstances.value.findIndex(i => i.effect_id === instanceId);
    if (index !== -1) {
      userEffectInstances.value.splice(index, 1);
    }
    
    return await invoke('delete_effect_instance', { instance_id: instanceId });
  };

  return {
    // State
    ownedEffects,
    userEffectInstances,
    marketplaceEffects,
    categories,
    loading,
    error,
    
    // Filters
    searchQuery,
    selectedCategory,
    priceRange,
    sortBy,
    
    // Computed
    filteredEffects,
    ownedEffectIds,
    
    // API methods
    fetchMarketplaceEffects,
    fetchOwnedEffects,
    purchaseEffect,
    
    // Effect management
    createEffectInstance,
    updateEffectInstance,
    deleteEffectInstance,
    
    // Usage
    triggerMarketplaceEffect,
    previewEffect,
    downloadEffectAssets,
    
    // Utilities
    getDefaultConfig
  };
};