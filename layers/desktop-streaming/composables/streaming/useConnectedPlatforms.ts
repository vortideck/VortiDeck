import { ref, computed } from 'vue';

/**
 * Connected social account from backend
 */
export interface ConnectedAccount {
  id: number;
  provider: string;  // 'twitch', 'youtube', 'kick', 'tiktok'
  uid: string;
  is_primary: boolean;
  extra_data?: {
    username?: string;
    name?: string;
    display_name?: string;
    email?: string;
    picture?: string;
  };
  token_status?: {
    error?: {
      action_required?: boolean;
    };
  };
}

/**
 * Platform with its connected accounts
 */
export interface PlatformWithAccounts {
  platform: 'twitch' | 'youtube' | 'kick' | 'tiktok';
  accounts: ConnectedAccount[];
}

// Global cache
const cachedAccounts = ref<ConnectedAccount[]>([]);
const lastFetch = ref<number | null>(null);
const isLoading = ref(false);

const CACHE_DURATION = 5 * 60 * 1000; // 5 minutes



/**
 * Composable for managing connected streaming platform accounts
 */
export function useConnectedPlatforms() {
  const config = useRuntimeConfig();

  /**
 * Get display name for platform (proper branding)
 */
  function getPlatformDisplayName(platform: string): string {
    const displayNames: Record<string, string> = {
      'twitch': 'Twitch',
      'youtube': 'YouTube',
      'kick': 'Kick',
      'tiktok': 'TikTok',
    };
    return displayNames[platform.toLowerCase()] || platform;
  }

  /**
   * Fetch connected accounts from backend
   */
  async function fetchAccounts(forceRefresh = false): Promise<ConnectedAccount[]> {
    // Use cache if fresh and not forcing refresh
    if (!forceRefresh && lastFetch.value && Date.now() - lastFetch.value < CACHE_DURATION) {
      console.log('📦 Using cached connected platforms');
      return cachedAccounts.value;
    }

    if (isLoading.value) {
      console.log('⏳ Already fetching connected platforms...');
      return cachedAccounts.value;
    }

    isLoading.value = true;

    try {
      console.log('🔄 Fetching connected platforms from backend...');

      const response = await useFetchInterceptor(
        `${config.public.API_URL}/api/v1/auth/social/connected`,
        {
          method: 'GET',
          headers: {
            'Content-Type': 'application/json',
          },
        }
      );

      if (response.status === 200) {
        const data = await response.json();

        if (data?.connected_accounts) {
          // Filter to only streaming platforms
          // Note: YouTube is stored as 'google' provider in the backend
          const streamingPlatforms = ['twitch', 'google', 'kick', 'tiktok'];
          const accounts = data.connected_accounts.filter((account: ConnectedAccount) =>
            streamingPlatforms.includes(account.provider)
          );

          cachedAccounts.value = accounts;
          lastFetch.value = Date.now();

          console.log(`✅ Loaded ${accounts.length} connected streaming accounts`);
          console.log('📋 Accounts:', accounts.map(a => `${a.provider} - ${a.extra_data?.username || a.extra_data?.email}`));
          return accounts;
        }
      }

      console.warn('⚠️ Failed to fetch connected accounts - status:', response.status);
      return [];
    } catch (error) {
      console.error('❌ Failed to fetch connected platforms:', error);
      return cachedAccounts.value; // Return cached data on error
    } finally {
      isLoading.value = false;
    }
  }

  /**
   * Invalidate cache (call this after connecting/disconnecting accounts)
   */
  function invalidateCache() {
    console.log('🗑️ Invalidating connected platforms cache');
    lastFetch.value = null;
  }

  /**
   * Get accounts grouped by platform
   */
  const platformsWithAccounts = computed((): PlatformWithAccounts[] => {
    const platforms: PlatformWithAccounts[] = [
      { platform: 'twitch', accounts: [] },
      { platform: 'youtube', accounts: [] },
      { platform: 'kick', accounts: [] },
      { platform: 'tiktok', accounts: [] },
    ];

    // Track seen account IDs to avoid duplicates
    const seenAccountIds = new Set<number>();

    cachedAccounts.value.forEach(account => {
      // Skip duplicates
      if (seenAccountIds.has(account.id)) {
        console.warn('⚠️ Duplicate account detected:', account.id, account.provider);
        return;
      }
      seenAccountIds.add(account.id);

      // Map 'google' provider to 'youtube' platform
      const platformName = account.provider === 'google' ? 'youtube' : account.provider;
      const platform = platforms.find(p => p.platform === platformName);
      if (platform) {
        platform.accounts.push(account);
      }
    });

    // Filter out platforms with no accounts for cleaner UI
    return platforms.filter(p => p.accounts.length > 0);
  });

  /**
   * Get accounts for a specific platform
   */
  function getAccountsForPlatform(platform: string): ConnectedAccount[] {
    return cachedAccounts.value.filter(account => account.provider === platform);
  }

  /**
   * Get display name for an account
   */
  function getAccountDisplayName(account: ConnectedAccount): string {
    if (account.extra_data?.username) return account.extra_data.username;
    if (account.extra_data?.display_name) return account.extra_data.display_name;
    if (account.extra_data?.name) return account.extra_data.name;
    if (account.extra_data?.email) return account.extra_data.email.split('@')[0];
    return account.uid;
  }

  /**
   * Check if account needs reconnection
   */
  function needsReconnection(account: ConnectedAccount): boolean {
    return account.token_status?.error?.action_required === true;
  }

  return {
    // State
    accounts: computed(() => cachedAccounts.value),
    platformsWithAccounts,
    isLoading: computed(() => isLoading.value),

    // Methods
    fetchAccounts,
    invalidateCache,
    getAccountsForPlatform,
    getAccountDisplayName,
    needsReconnection,
    getPlatformDisplayName
  };
}
