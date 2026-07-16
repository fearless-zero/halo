// Display names for integration ids, shared by the setup wizard and settings.
export const INTEGRATION_LABELS: Record<string, string> = {
  markdown: "Markdown export",
  obsidian: "Obsidian",
  clipboard: "Clipboard",
  notion: "Notion",
  slack: "Slack",
  webhook: "Webhook",
  "google-calendar": "Google Calendar",
  "apple-calendar": "Apple Calendar",
  "microsoft-calendar": "Microsoft Calendar",
};

export function integrationLabel(id: string): string {
  return INTEGRATION_LABELS[id] ?? id;
}
