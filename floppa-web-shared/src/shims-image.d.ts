// Asset imports resolved by the consuming app's bundler (Vite) to a URL string.
declare module '*.png' {
  const src: string
  export default src
}
