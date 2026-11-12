// Small building blocks shared by every screen, in the Mitigata green on white.
import type { ButtonHTMLAttributes, ReactNode } from "react";

export function Card({ children, className = "" }: { children: ReactNode; className?: string }) {
  return (
    <section className={`rounded-2xl border border-line bg-white p-4 shadow-sm sm:p-5 ${className}`}>
      {children}
    </section>
  );
}

export function Button({
  variant = "primary",
  className = "",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "primary" | "quiet" }) {
  const look =
    variant === "primary"
      ? "bg-brand text-ink hover:bg-brand-dark hover:text-white"
      : "border border-line bg-white text-ink hover:border-brand";
  return (
    <button
      type="button"
      className={`min-h-11 rounded-xl px-4 py-2 font-semibold transition disabled:opacity-50 ${look} ${className}`}
      {...props}
    />
  );
}

type Tone = "info" | "good" | "warn" | "bad";
const TONES: Record<Tone, string> = {
  info: "border-line bg-brand-tint text-ink",
  good: "border-brand bg-brand-tint text-ink",
  warn: "border-amber-300 bg-amber-50 text-amber-950",
  bad: "border-red-300 bg-red-50 text-red-950",
};

export function Notice({ tone = "info", children }: { tone?: Tone; children: ReactNode }) {
  return (
    <p
      role={tone === "bad" ? "alert" : "status"}
      className={`rounded-xl border px-3 py-2 text-sm ${TONES[tone]}`}
    >
      {children}
    </p>
  );
}

/** One choice out of several, as large tappable chips (icon first, so it works without reading). */
export function Pills<T extends string>({
  legend,
  options,
  value,
  onChange,
  render,
}: {
  legend: string;
  options: readonly T[];
  value: T | null;
  onChange: (value: T) => void;
  render: (option: T) => string;
}) {
  return (
    <fieldset>
      <legend className="mb-2 font-semibold">{legend}</legend>
      <div className="flex flex-wrap gap-2">
        {options.map((option) => {
          const chosen = option === value;
          return (
            <button
              key={option}
              type="button"
              aria-pressed={chosen}
              onClick={() => onChange(option)}
              className={`min-h-11 rounded-full border px-3 py-2 text-sm transition ${
                chosen
                  ? "border-brand bg-brand font-semibold text-ink"
                  : "border-line bg-white hover:border-brand"
              }`}
            >
              {render(option)}
            </button>
          );
        })}
      </div>
    </fieldset>
  );
}

/** A label above its control; `id` must be the control's id. */
export function Field({ id, label, children }: { id: string; label: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-2">
      <label htmlFor={id} className="font-semibold">
        {label}
      </label>
      {children}
    </div>
  );
}

export const INPUT =
  "min-h-11 w-full rounded-xl border border-line bg-white px-3 py-2 text-base outline-none focus:border-brand";
