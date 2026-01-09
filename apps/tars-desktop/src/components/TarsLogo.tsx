interface TarsLogoProps {
  className?: string;
  size?: number;
}

export function TarsLogo({ className = '', size = 32 }: TarsLogoProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 64 64"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      {/* Main monolith body */}
      <rect
        x="16"
        y="4"
        width="32"
        height="56"
        rx="2"
        fill="url(#metallic-gradient)"
      />

      {/* Segment lines */}
      <line x1="16" y1="16" x2="48" y2="16" stroke="#1a1a1a" strokeWidth="1" />
      <line x1="16" y1="28" x2="48" y2="28" stroke="#1a1a1a" strokeWidth="1" />
      <line x1="16" y1="40" x2="48" y2="40" stroke="#1a1a1a" strokeWidth="1" />
      <line x1="16" y1="52" x2="48" y2="52" stroke="#1a1a1a" strokeWidth="1" />

      {/* Vertical segment line */}
      <line x1="32" y1="4" x2="32" y2="60" stroke="#1a1a1a" strokeWidth="0.5" />

      {/* Display screen */}
      <rect
        x="20"
        y="20"
        width="24"
        height="8"
        rx="1"
        fill="#0a0a0a"
      />

      {/* Amber status indicators */}
      <circle cx="24" cy="34" r="2" fill="#d4a574" />
      <circle cx="32" cy="34" r="2" fill="#d4a574" />
      <circle cx="40" cy="34" r="2" fill="#d4a574" />

      {/* Subtle reflection highlight */}
      <rect
        x="17"
        y="5"
        width="8"
        height="54"
        fill="url(#highlight-gradient)"
        opacity="0.3"
      />

      <defs>
        <linearGradient id="metallic-gradient" x1="16" y1="4" x2="48" y2="60" gradientUnits="userSpaceOnUse">
          <stop offset="0%" stopColor="#5a5a5a" />
          <stop offset="30%" stopColor="#4a4a4a" />
          <stop offset="70%" stopColor="#3a3a3a" />
          <stop offset="100%" stopColor="#2a2a2a" />
        </linearGradient>
        <linearGradient id="highlight-gradient" x1="17" y1="5" x2="25" y2="5" gradientUnits="userSpaceOnUse">
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
      {/* Simplified monolith */}
      <rect
        x="6"
        y="2"
        width="12"
        height="20"
        rx="1"
        fill="currentColor"
        opacity="0.8"
      />
      {/* Amber dot */}
      <circle cx="12" cy="12" r="2" fill="#d4a574" />
    </svg>
  );
}
