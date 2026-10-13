import type { ButtonHTMLAttributes, InputHTMLAttributes, ReactNode, TextareaHTMLAttributes } from "react";

const field =
  "w-full rounded-md border border-line bg-transparent px-2.5 py-1.5 outline-none focus:border-accent placeholder:text-muted";

export function Field({ label, hint, children }: { label: string; hint?: ReactNode; children: ReactNode }) {
  return (
    <label className="block space-y-1">
      <span className="text-xs font-medium text-muted">{label}</span>
      {children}
      {hint && <span className="block text-xs text-muted">{hint}</span>}
    </label>
  );
}

export function TextInput(props: InputHTMLAttributes<HTMLInputElement>) {
  return <input spellCheck={false} {...props} className={`${field} ${props.className ?? ""}`} />;
}

export function TextArea(props: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea spellCheck={false} {...props} className={`${field} resize-y ${props.className ?? ""}`} />;
}

export function Button({
  variant = "secondary",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "primary" | "secondary" | "danger" }) {
  const styles = {
    primary: "bg-accent text-accent-fg",
    secondary: "border border-line hover:bg-surface-2",
    danger: "border border-line text-danger hover:bg-surface-2",
  }[variant];
  return (
    <button
      type="button"
      {...props}
      className={`rounded-md px-3 py-1.5 disabled:opacity-50 ${styles} ${props.className ?? ""}`}
    />
  );
}

export function Card({ children }: { children: ReactNode }) {
  return <div className="space-y-3 rounded-xl border border-line p-4">{children}</div>;
}

export function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <label className="flex cursor-pointer items-center gap-2">
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} className="size-4 accent-[var(--accent)]" />
      {label}
    </label>
  );
}

/** An id not already in `taken`. */
export function uniqueId(base: string, taken: string[]): string {
  let id = base;
  for (let n = 2; taken.includes(id); n++) id = `${base}-${n}`;
  return id;
}
