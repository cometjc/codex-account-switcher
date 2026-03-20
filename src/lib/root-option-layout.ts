export interface RootSelectionOptionInput {
  indicator: string;
  profile: string;
  delta: string;
}

export function renderSelectionOptionLabel(input: RootSelectionOptionInput): string {
  const parts = [input.indicator, input.profile, input.delta].filter(Boolean);
  return parts.join(" ");
}
