import { useEffect, useState } from "react";

export function usePlannerWindowWidth(defaultWidth = 1440) {
  const [windowWidth, setWindowWidth] = useState<number>(() =>
    typeof window === "undefined" ? defaultWidth : window.innerWidth,
  );

  useEffect(() => {
    const updateWidth = () => setWindowWidth(window.innerWidth);
    window.addEventListener("resize", updateWidth);
    return () => window.removeEventListener("resize", updateWidth);
  }, []);

  return windowWidth;
}
