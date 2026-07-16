import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import * as Icons from "./icons";

describe("icons", () => {
  it("every exported icon renders an svg", () => {
    for (const Icon of Object.values(Icons)) {
      const { container, unmount } = render(<Icon />);
      expect(container.querySelector("svg")).not.toBeNull();
      unmount();
    }
  });
});
