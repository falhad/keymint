const HUES: Record<string, number> = {
  AI: 265,
  "Custom Prompt": 295,
  Text: 200,
  Web: 150,
  Shell: 30,
  Keymint: 220,
};

/** Category-tinted square with the command's first letter. Cheap and dependency-free. */
export function Badge({ title, category }: { title: string; category: string }) {
  const hue = HUES[category] ?? 220;
  return (
    <span
      aria-hidden
      className="grid size-7 shrink-0 place-items-center rounded-md text-[13px] font-semibold"
      style={{ background: `hsl(${hue} 70% 55% / 0.16)`, color: `hsl(${hue} 70% 55%)` }}
    >
      {title.trim().charAt(0).toUpperCase()}
    </span>
  );
}
