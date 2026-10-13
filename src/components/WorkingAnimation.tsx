import { useEffect, useRef, useState } from "react";
import animation from "../assets/working.json";

/**
 * Lottie loop shown while the AI works. The player is loaded only when this
 * mounts so it costs nothing at startup; if it fails to load, a CSS spinner is used.
 */
export function WorkingAnimation({ className = "" }: { className?: string }) {
  const host = useRef<HTMLDivElement>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    let destroy: (() => void) | undefined;
    import("lottie-web/build/player/lottie_light")
      .then(({ default: lottie }) => {
        if (cancelled || !host.current) return;
        const anim = lottie.loadAnimation({
          container: host.current,
          renderer: "svg",
          loop: true,
          autoplay: true,
          animationData: animation,
        });
        destroy = () => anim.destroy();
      })
      .catch((e) => {
        console.error("lottie failed to load", e);
        if (!cancelled) setFailed(true);
      });
    return () => {
      cancelled = true;
      destroy?.();
    };
  }, []);

  if (failed) {
    return <span className="inline-block size-4 animate-spin rounded-full border-2 border-line border-t-accent" />;
  }
  return <div ref={host} aria-hidden className={`h-10 w-[120px] ${className}`} />;
}
