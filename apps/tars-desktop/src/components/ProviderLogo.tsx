import openaiLogo from '../assets/provider-logos/openai.svg';
import anthropicLogo from '../assets/provider-logos/anthropic.svg';
import geminiLogo from '../assets/provider-logos/gemini.svg';
import deepseekLogo from '../assets/provider-logos/deepseek.svg';
import braveLogo from '../assets/provider-logos/brave.svg';
import elevenlabsLogo from '../assets/provider-logos/elevenlabs.svg';
import groqLogo from '../assets/provider-logos/groq.svg';
import mistralLogo from '../assets/provider-logos/mistral.svg';
import xaiLogo from '../assets/provider-logos/xai.svg';
import openrouterLogo from '../assets/provider-logos/openrouter.svg';
import perplexityLogo from '../assets/provider-logos/perplexity.svg';

// Maps tars_providers::ProviderId string values to bundled brand SVGs. Keys
// must stay in sync with `ProviderId::as_str` on the Rust side.
const PROVIDER_LOGOS: Record<string, string> = {
  openai: openaiLogo,
  anthropic: anthropicLogo,
  gemini: geminiLogo,
  deepseek: deepseekLogo,
  'brave-search': braveLogo,
  elevenlabs: elevenlabsLogo,
  groq: groqLogo,
  mistral: mistralLogo,
  xai: xaiLogo,
  openrouter: openrouterLogo,
  perplexity: perplexityLogo,
};

export interface ProviderLogoProps {
  providerId: string;
  providerName: string;
  className?: string;
}

export function ProviderLogo({ providerId, providerName, className }: ProviderLogoProps) {
  const src = PROVIDER_LOGOS[providerId];
  if (!src) {
    // Fallback for unknown providers: first-letter badge so the header layout
    // stays consistent rather than collapsing when a new provider ships
    // before its logo is bundled.
    return (
      <div
        className={`flex items-center justify-center rounded bg-muted text-xs font-semibold text-muted-foreground ${className ?? ''}`}
        aria-hidden="true"
      >
        {providerName.charAt(0).toUpperCase()}
      </div>
    );
  }
  return <img src={src} alt="" aria-hidden="true" className={className} draggable={false} />;
}
