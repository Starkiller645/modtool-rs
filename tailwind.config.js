/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.rs"],
  theme: {
    extend: {
      fontSize: {
        huge: ["6rem", "9rem"],
      },
    },
  },
  plugins: [],
};
