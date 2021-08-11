const colors = require("tailwindcss/colors");

module.exports = {
  // Uncomment the line below to enable the experimental Just-in-Time ("JIT") mode.
  // https://tailwindcss.com/docs/just-in-time-mode
  // mode: "jit",
  theme: {
    colors: {
      transparent: "transparent",
      white: colors.white,
      black: colors.black,
      gray: colors.gray,
      cyan: colors.cyan,
      background: {
        light: colors.white,
        DEFAULT: "#f9fafc",
        dark: "#ecedef",
      },
      orange: {
        DEFAULT: "#FF7477",
      },
    },
    extend: {},
  },
  variants: {},
  plugins: [],
  purge: {
    // Filenames to scan for classes
    content: [
      "./src/**/*.html",
      "./src/**/*.js",
      "./src/**/*.jsx",
      "./src/**/*.ts",
      "./src/**/*.tsx",
      "./public/index.html",
    ],
    // Options passed to PurgeCSS
    options: {
      // Whitelist specific selectors by name
      // safelist: [],
    },
  },
};
