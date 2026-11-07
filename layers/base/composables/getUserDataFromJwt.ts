import { Buffer } from 'buffer';
export const getUserDataFromJwt = (token: string) => {

    // Decode the payload (second part of the token)
    if (!token) return null;
    
    try {
        const tokenParts = token.split('.');
        if (tokenParts.length !== 3) {
            throw new Error('Invalid token format');
        }
        
        // Decode the payload (second part of the token)
        const payload = JSON.parse(Buffer.from(tokenParts[1], 'base64').toString());
      
        return payload.user;
    } catch (error) {
        console.error('Error decoding JWT token:', error);
  
        return null;
    }

};




