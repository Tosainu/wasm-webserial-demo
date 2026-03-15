export default {
  plugins: ['prettier-plugin-tailwindcss'],
  semi: true,
  singleQuote: true,
  printWidth: 100,
  overrides: [
    {
      files: './**/*.html',
      options: {
        printWidth: 256,
      },
    },
  ],
};
