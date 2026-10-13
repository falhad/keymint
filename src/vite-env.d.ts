/// <reference types="vite/client" />

// lottie-web ships types for its main entry only; the light build has the same API.
declare module "lottie-web/build/player/lottie_light" {
  import lottie from "lottie-web";
  export default lottie;
}
