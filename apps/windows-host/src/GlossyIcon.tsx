import React, { useId } from "react";

export type IconPalette =
  | "violet"
  | "lavender"
  | "indigo"
  | "blue"
  | "neutral"
  | "amber"
  | "green"
  | "red";

export type IconName =
  | "home"
  | "monitor"
  | "devices"
  | "bolt"
  | "pulse"
  | "sliders"
  | "wifi"
  | "shield"
  | "lock"
  | "info"
  | "eye"
  | "phone"
  | "check"
  | "plus"
  | "laptop"
  | "keyboard"
  | "cursor";

const PALETTES: Record<IconPalette, [string, string]> = {
  violet: ["#8D7BFF", "#382098"],
  lavender: ["#B9AEFF", "#4D33C4"],
  indigo: ["#7C64E6", "#1F1266"],
  blue: ["#8FA8FF", "#3A4FD0"],
  neutral: ["#9CA3AF", "#374151"],
  amber: ["#FCD34D", "#D97706"],
  green: ["#86EFAC", "#15803D"],
  red: ["#FCA5A5", "#DC2626"],
};

const DEFAULT_PALETTES: Record<IconName, IconPalette> = {
  home: "violet",
  monitor: "violet",
  devices: "lavender",
  bolt: "violet",
  pulse: "blue",
  sliders: "neutral",
  wifi: "blue",
  shield: "indigo",
  lock: "amber",
  info: "blue",
  eye: "lavender",
  phone: "lavender",
  check: "green",
  plus: "violet",
  laptop: "blue",
  keyboard: "neutral",
  cursor: "violet",
};

const GLYPHS: Record<IconName, React.ReactNode> = {
  home: <path d="M3 11l9-7 9 7v9a1 1 0 0 1-1 1h-5v-6h-6v6H4a1 1 0 0 1-1-1z" />,
  monitor: (
    <>
      <rect x="2" y="3" width="20" height="13" rx="2" />
      <path d="M8 21h8M12 16v5" />
    </>
  ),
  devices: (
    <>
      <rect x="2" y="5" width="13" height="10" rx="2" />
      <rect x="17" y="9" width="5" height="11" rx="1" />
    </>
  ),
  bolt: <path d="M13 2L4 14h7l-1 8 9-12h-7z" />,
  pulse: <path d="M3 12h4l3-8 4 16 3-8h4" />,
  sliders: (
    <>
      <path d="M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12M20 18h0" />
      <circle cx="16" cy="6" r="2" />
      <circle cx="10" cy="12" r="2" />
      <circle cx="18" cy="18" r="2" />
    </>
  ),
  wifi: (
    <>
      <path d="M2 9a15 15 0 0 1 20 0M5 12.5a10 10 0 0 1 14 0M8.5 16a5 5 0 0 1 7 0" />
      <circle cx="12" cy="20" r="1" />
    </>
  ),
  shield: <path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6z" />,
  lock: (
    <>
      <rect x="5" y="11" width="14" height="10" rx="2" />
      <path d="M8 11V7a4 4 0 0 1 8 0v4" />
    </>
  ),
  info: (
    <>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 11v5M12 8h.01" />
    </>
  ),
  eye: (
    <>
      <path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z" />
      <circle cx="12" cy="12" r="3" />
    </>
  ),
  phone: (
    <>
      <rect x="6" y="2" width="12" height="20" rx="3" />
      <path d="M11 18h2" />
    </>
  ),
  check: <path d="M5 12.5l4.5 4.5L19 7.5" />,
  plus: <path d="M12 5v14M5 12h14" />,
  laptop: (
    <>
      <rect x="4" y="5" width="16" height="11" rx="2" />
      <path d="M2 19h20" />
    </>
  ),
  keyboard: (
    <>
      <rect x="2" y="6" width="20" height="12" rx="2" />
      <path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M7 14h10" />
    </>
  ),
  cursor: <path d="M5 3l14 8-6 2-3 6z" />,
};

interface GlossyIconProps {
  name: IconName;
  palette?: IconPalette;
  size?: number;
  className?: string;
  style?: React.CSSProperties;
}

export const GlossyIcon: React.FC<GlossyIconProps> = ({
  name,
  palette,
  size = 32,
  className = "",
  style = {},
}) => {
  const rawId = useId();
  const id = `gi_${rawId.replace(/[:]/g, "_")}`;
  const effectivePalette = palette || DEFAULT_PALETTES[name] || "violet";
  const [c1, c2] = PALETTES[effectivePalette];

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 64 64"
      aria-hidden="true"
      className={`tile ${className}`}
      style={{
        flexShrink: 0,
        display: "inline-block",
        verticalAlign: "middle",
        filter: "drop-shadow(0 4px 10px rgba(0,0,0,0.35))",
        ...style,
      }}
    >
      <defs>
        <linearGradient id={`${id}_grad`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={c1} />
          <stop offset="100%" stopColor={c2} />
        </linearGradient>
        <linearGradient id={`${id}_gloss`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#FFFFFF" stopOpacity="0.58" />
          <stop offset="100%" stopColor="#FFFFFF" stopOpacity="0" />
        </linearGradient>
      </defs>
      <rect width="64" height="64" rx="18" fill={`url(#${id}_grad)`} />
      <path
        d="M1.5 18Q1.5 2 18 2H46Q62.5 2 62.5 18V31Q32 42 1.5 31Z"
        fill={`url(#${id}_gloss)`}
      />
      <rect
        x="0.75"
        y="0.75"
        width="62.5"
        height="62.5"
        rx="17.5"
        fill="none"
        stroke="rgba(255,255,255,0.24)"
        strokeWidth="1.5"
      />
      <g
        transform="translate(16 16) scale(1.333)"
        fill="none"
        stroke="#FFFFFF"
        strokeWidth="1.7"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        {GLYPHS[name]}
      </g>
    </svg>
  );
};
