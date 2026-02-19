export function HashChip({
  hash,
  href,
  prefix = "0x",
}: {
  hash: string;
  href?: string;
  prefix?: string;
}) {
  const short = `${prefix}${hash.slice(0, 6)}\u2026${hash.slice(-6)}`;
  const cls = "font-mono text-[13px] transition-opacity hover:opacity-80";
  const style = { color: "var(--text-dim)" };
  if (href)
    return (
      <a href={href} className={cls} style={{ ...style, color: "var(--blue-400)" }}>
        {short}
      </a>
    );
  return (
    <span className={cls} style={style}>
      {short}
    </span>
  );
}
