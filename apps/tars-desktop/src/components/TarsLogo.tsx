interface TarsLogoProps {
  className?: string;
  size?: number;
}

export function TarsLogo({ className = '', size = 32 }: TarsLogoProps) {
  // Scale factor based on size (viewBox is 64x64)
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 64 64"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      {/* Four articulated segments like TARS robot */}
      {/* Segment 1 - leftmost */}
      <rect x="8" y="6" width="10" height="52" rx="1.5" fill="url(#segment-gradient-1)" />

      {/* Segment 2 */}
      <rect x="20" y="4" width="10" height="56" rx="1.5" fill="url(#segment-gradient-2)" />

      {/* Segment 3 */}
      <rect x="32" y="4" width="10" height="56" rx="1.5" fill="url(#segment-gradient-2)" />

      {/* Segment 4 - rightmost */}
      <rect x="44" y="6" width="10" height="52" rx="1.5" fill="url(#segment-gradient-1)" />

      {/* Braille-like dots on segment 2 - matching TARS pattern */}
      {/* Top group */}
      <circle cx="25" cy="18" r="1.8" fill="#d4a574" />
      <circle cx="25" cy="24" r="1.8" fill="#d4a574" />
      <circle cx="25" cy="30" r="1.8" fill="#d4a574" />

      {/* Bottom group */}
      <circle cx="25" cy="40" r="1.8" fill="#d4a574" />
      <circle cx="25" cy="46" r="1.8" fill="#d4a574" />

      {/* Braille-like dots on segment 3 - mirrored pattern */}
      {/* Top group */}
      <circle cx="37" cy="18" r="1.8" fill="#d4a574" />
      <circle cx="37" cy="24" r="1.8" fill="#d4a574" />
      <circle cx="37" cy="30" r="1.8" fill="#d4a574" />

      {/* Bottom group */}
      <circle cx="37" cy="40" r="1.8" fill="#d4a574" />
      <circle cx="37" cy="46" r="1.8" fill="#d4a574" />

      {/* Display screen area on center segments */}
      <rect x="22" y="10" width="18" height="4" rx="0.5" fill="#0a0a0a" opacity="0.6" />

      {/* Subtle highlights on segments */}
      <rect x="8" y="6" width="2" height="52" rx="0.5" fill="url(#highlight)" opacity="0.2" />
      <rect x="20" y="4" width="2" height="56" rx="0.5" fill="url(#highlight)" opacity="0.25" />
      <rect x="32" y="4" width="2" height="56" rx="0.5" fill="url(#highlight)" opacity="0.25" />
      <rect x="44" y="6" width="2" height="52" rx="0.5" fill="url(#highlight)" opacity="0.2" />

      <defs>
        <linearGradient id="segment-gradient-1" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#4a4a4a" />
          <stop offset="50%" stopColor="#3a3a3a" />
          <stop offset="100%" stopColor="#2a2a2a" />
        </linearGradient>
        <linearGradient id="segment-gradient-2" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#555555" />
          <stop offset="50%" stopColor="#404040" />
          <stop offset="100%" stopColor="#303030" />
        </linearGradient>
        <linearGradient id="highlight" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stopColor="#ffffff" />
          <stop offset="100%" stopColor="#ffffff" stopOpacity="0" />
        </linearGradient>
      </defs>
    </svg>
  );
}

export function TarsLogoMark({ className = '', size = 24 }: TarsLogoProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      {/* Simplified 4-segment design */}
      <rect x="3" y="3" width="4" height="18" rx="0.5" fill="currentColor" opacity="0.7" />
      <rect x="8" y="2" width="4" height="20" rx="0.5" fill="currentColor" opacity="0.85" />
      <rect x="13" y="2" width="4" height="20" rx="0.5" fill="currentColor" opacity="0.85" />
      <rect x="18" y="3" width="4" height="18" rx="0.5" fill="currentColor" opacity="0.7" />

      {/* Braille dots */}
      <circle cx="10" cy="8" r="1" fill="#d4a574" />
      <circle cx="10" cy="12" r="1" fill="#d4a574" />
      <circle cx="15" cy="8" r="1" fill="#d4a574" />
      <circle cx="15" cy="12" r="1" fill="#d4a574" />
    </svg>
  );
}
