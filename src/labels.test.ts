import { describe, expect, it } from "vitest";
import { INTEGRATION_LABELS, integrationLabel } from "./labels";

describe("integrationLabel", () => {
  it("returns friendly names for known ids", () => {
    expect(integrationLabel("slack")).toBe("Slack");
    expect(integrationLabel("google-calendar")).toBe("Google Calendar");
  });

  it("falls back to the raw id when unknown", () => {
    expect(integrationLabel("mystery")).toBe("mystery");
  });

  it("covers every integration", () => {
    expect(Object.keys(INTEGRATION_LABELS)).toContain("microsoft-calendar");
  });
});
