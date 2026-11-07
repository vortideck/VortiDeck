import vortideck, {
  createPlugin,
  defineNode,
  FlowParams,
} from "@vortideck/plugin-sdk";

export const { execute, register, get_metadata } = createPlugin({
  nodes: {
    uppercase: defineNode({
      metadata: {
        id: "contract-test:uppercase",
        name: "Uppercase",
        description: "Compile the documented minimal contribution",
        category: "text",
        type_color: "#4A90E2",
      },
      execute(input) {
        vortideck.log("Uppercase node executed");
        const text = new FlowParams(input).getInput();
        return { success: true, output: text.toUpperCase() };
      },
    }),
  },
});
