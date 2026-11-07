
import { openUrl } from "@tauri-apps/plugin-opener";

// Generate a random code_verifier (43-128 characters)
const generateCodeVerifier = () => {
    const array = new Uint8Array(64);
    crypto.getRandomValues(array);
    return btoa(String.fromCharCode.apply(null, array))
      .replace(/[\+\/=]/g, "") // Remove special chars
      .substring(0, 128);
};

// Hash the code_verifier with SHA256 and encode in base64
const generateCodeChallenge = async (codeVerifier) => {
    const encoder = new TextEncoder();
    const data = encoder.encode(codeVerifier);
    const hashed = await crypto.subtle.digest("SHA-256", data);
    return btoa(String.fromCharCode(...new Uint8Array(hashed))).replace(/[\+\/=]/g, ""); // URL-safe
};
  

export const useLoginWithGoogle = async () => {

    const config = useRuntimeConfig();

    const clientId = config.public.GOOGLE_LOGIN_CLIENT_ID

    

    if (!clientId) {
      console.error("Could not get client ID from redirect URL");
      throw new Error('Google OAuth not configured. Please contact support.')

    }
  
    // Generate PKCE values
    // const codeVerifier = generateCodeVerifier();
    // const codeChallenge = await generateCodeChallenge(codeVerifier);
  
    // Save the code_verifier in IndexedDB / Secure Storage
    // localStorage.setItem("pkce_code_verifier", codeVerifier);
  
    // Prepare Google Auth URL with PKCE

    console.log(config.public.API_URL);
    
    const params = new URLSearchParams({
      // Runtime config is only typed as string when the env var exists at build time.
      client_id: String(clientId),
      redirect_uri: `${config.public.API_URL}/api/v1/auth/google/callback`,
      response_type: "code",
      scope: "openid email profile",
      access_type: "offline",
      prompt: "consent",
      // code_challenge: codeChallenge,
      // code_challenge_method: "S256",
    });
  
  
    const googleAuthUrl = `https://accounts.google.com/o/oauth2/v2/auth?${params.toString()}`;
  
    console.log(googleAuthUrl);
    await openUrl(googleAuthUrl);
  


}

