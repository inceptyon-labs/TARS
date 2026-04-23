import { ArrowLeft } from 'lucide-react';
import { useLocation, useNavigate } from 'react-router-dom';
import { Button } from './ui/button';

type NavigationState = {
  returnTo?: string;
  returnLabel?: string;
};

export function PageBackButton() {
  const navigate = useNavigate();
  const location = useLocation();
  const state = (location.state as NavigationState | null) ?? null;
  const returnTo = state?.returnTo;

  if (!returnTo) {
    return null;
  }

  return (
    <Button variant="ghost" size="sm" onClick={() => navigate(returnTo)}>
      <ArrowLeft className="h-4 w-4" />
      {state.returnLabel ?? 'Back'}
    </Button>
  );
}
