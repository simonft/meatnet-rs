/** @type {import('tailwindcss').Config} */
module.exports = {
  content: {
    files: ["*.html", "./src/**/*.rs"],
    extract: {
      // Support for leptos class:classname=predicate and 
      // class=("classname", predicate) syntax.
      // Without this the tuple syntax works but not 
      // the inline syntax.
      rs: (content) => content
        .match(/(?<=class[:=]\(?"?)[-\w: ]+/g)
        ?.flatMap(s => s.split(' ')) ?? []
    }
  },
  theme: {
    extend: {},
  },
  plugins: [],
}

