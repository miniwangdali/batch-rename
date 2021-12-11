const {
  override,
  adjustStyleLoaders,
  addBabelPlugin,
  addWebpackPlugin
} = require("customize-cra");
const LodashModuleReplacementPlugin = require("lodash-webpack-plugin");

module.exports = override(
  adjustStyleLoaders((rule) => {
    if (rule.test.toString().includes("scss")) {
      rule.use.push({
        loader: require.resolve("sass-resources-loader"),
        options: {
          resources: [
            "./src/scss/animations.scss",
            "./src/scss/dimensions.scss",
            "./src/scss/typography.scss",
            "./src/scss/variables.scss"
          ]
        }
      });
    }
  }),
  addBabelPlugin("lodash"),
  addWebpackPlugin(new LodashModuleReplacementPlugin())
);
