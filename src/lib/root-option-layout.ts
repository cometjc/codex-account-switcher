export interface RootSelectionOptionInput {
  indicator: string;
  profile: string;
  delta: string;
  influence?: string;
}

export function renderSelectionOptionLabel(input: RootSelectionOptionInput): string {
  const parts = [input.indicator, input.profile, input.delta, input.influence].filter(Boolean);
  return parts.join(" ");
}
