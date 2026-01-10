import { HelpCircle } from 'lucide-react';
import { Link } from 'react-router-dom';
import { CASE_SECTIONS } from '../pages/CasePage';

interface HelpButtonProps {
  section: keyof typeof CASE_SECTIONS;
  className?: string;
}

/**
 * A help button that links to the relevant CASE knowledge base section
 */
export function HelpButton({ section, className = '' }: HelpButtonProps) {
  const sectionId = CASE_SECTIONS[section];

  return (
    <Link
      to={`/case/${sectionId}`}
      className={`p-1.5 rounded-md text-muted-foreground hover:text-foreground hover:bg-muted transition-all ${className}`}
      title={`Learn about ${section.toLowerCase()}`}
    >
      <HelpCircle className="h-4 w-4" />
    </Link>
  );
}
