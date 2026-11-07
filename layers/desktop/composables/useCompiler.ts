export function useCompiler() {}

// import { parse, compileTemplate, compileScript, compileStyle } from "@vue/compiler-sfc";

// const compileVueFile = (vueFileContent) => {
//   // Step 1: Parse the Vue file content
//   const { descriptor } = parse(vueFileContent);

//   // Step 2: Compile the template
//   const template = compileTemplate({
//     source: descriptor.template.content,
//     filename: "inline-template.vue",
//   });

//   // Step 3: Compile the script
//   const script = compileScript(descriptor, {});

//   // Step 4: Compile the styles
//   const styles = descriptor.styles
//     .map((styleBlock) =>
//       compileStyle({
//         source: styleBlock.content,
//         filename: "inline-style.vue",
//       })
//     )
//     .map((compiledStyle) => compiledStyle.code)
//     .join("\n");

//   // Combine the compiled content into separate strings
//   const compiledHtml = template.code;
//   const compiledJs = `
//     ${script.content}
//     ${template.code}
//     export default {
//       ...script,
//       render: render,
//     }
//   `;
//   return {
//     html: compiledHtml,
//     js: compiledJs,
//     css: styles,
//   };
// };
