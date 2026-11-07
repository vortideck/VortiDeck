import { ref } from 'vue';

export const useHoverStyles = (theme) => {
  const hoverStyles = ref('');

  const extractHoverStyles = (cssString) => {
    const cleanedString = cssString.trim();
    
    // Find the index of &:hover
    const hoverIndex = cleanedString.indexOf('&:hover');
    
    if (hoverIndex === -1) {
      return '';
    }

    // Find the opening brace after &:hover
    const openBraceIndex = cleanedString.indexOf('{', hoverIndex);
    if (openBraceIndex === -1) {
      return '';
    }

    // Handle the case where the closing brace might be missing
    // Extract everything after the opening brace
    let hoverStyles = cleanedString.substring(openBraceIndex + 1).trim();
    
    // Look for a closing brace
    const closeBraceIndex = hoverStyles.indexOf('}');
    
    // If closing brace exists, extract only up to that point
    if (closeBraceIndex !== -1) {
      hoverStyles = hoverStyles.substring(0, closeBraceIndex).trim();
    }
    
    return hoverStyles;
  };

  const applyHoverStyles = () => {
    if (!theme?.knobCss) return;
    
    // Add some debugging to check what's being processed
    // console.log('Processing CSS:', theme.knobCss);
    
    const styles = extractHoverStyles(theme.knobCss);
    // console.log('Extracted hover styles:', styles);
    
    hoverStyles.value = styles || '';
  };

  // Call applyHoverStyles initially
  applyHoverStyles();

  // Return both the hoverStyles ref and the applyHoverStyles function
  return {
    hoverStyles,
    applyHoverStyles,
  };
};