// How it works: the trust and privacy rules in plain words. The text uses a small markdown
// subset (**bold**, "- " bullets, blank lines between blocks), rendered here without a library.
import type { ReactNode } from "react";
import { Card } from "../components/ui";
import { useSettings } from "../lib/settings";

function inline(text: string): ReactNode[] {
  return text
    .split(/\*\*(.+?)\*\*/g)
    .map((part, i) => (i % 2 === 1 ? <strong key={part}>{part}</strong> : part));
}

export function renderBlocks(source: string): ReactNode[] {
  return source.split("\n\n").map((block) => {
    const lines = block.split("\n");
    if (lines.every((l) => l.startsWith("- "))) {
      return (
        <ul key={block} className="ml-5 list-disc space-y-1">
          {lines.map((l) => (
            <li key={l}>{inline(l.slice(2))}</li>
          ))}
        </ul>
      );
    }
    return <p key={block}>{inline(block)}</p>;
  });
}

export function How() {
  const { tr } = useSettings();
  return <Card className="flex flex-col gap-3 leading-relaxed">{renderBlocks(tr("how_body"))}</Card>;
}
