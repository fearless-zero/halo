import { describe, expect, it } from "vitest";
import { renderMarkdown } from "./markdown";

describe("renderMarkdown", () => {
  it("renders headings", () => {
    expect(renderMarkdown("# Title")).toContain("<h1>Title</h1>");
    expect(renderMarkdown("### Small")).toContain("<h3>Small</h3>");
  });

  it("renders unordered and ordered lists", () => {
    const ul = renderMarkdown("- a\n- b");
    expect(ul).toContain("<ul>");
    expect(ul).toContain("<li>a</li>");
    const ol = renderMarkdown("1. first\n2. second");
    expect(ol).toContain("<ol>");
    expect(ol).toContain("<li>first</li>");
  });

  it("renders bold, italic and inline code", () => {
    expect(renderMarkdown("**bold**")).toContain("<strong>bold</strong>");
    expect(renderMarkdown("*em*")).toContain("<em>em</em>");
    expect(renderMarkdown("`code`")).toContain("<code>code</code>");
  });

  it("escapes HTML to prevent injection", () => {
    const out = renderMarkdown("<script>alert('x')</script>");
    expect(out).not.toContain("<script>");
    expect(out).toContain("&lt;script&gt;");
  });

  it("renders links with safe attributes", () => {
    const out = renderMarkdown("[site](https://example.com)");
    expect(out).toContain('href="https://example.com"');
    expect(out).toContain('rel="noreferrer noopener"');
  });

  it("renders fenced code blocks without inner formatting", () => {
    const out = renderMarkdown("```\n**not bold**\n```");
    expect(out).toContain("<pre><code>");
    expect(out).toContain("**not bold**");
  });

  it("renders paragraphs and blockquotes", () => {
    expect(renderMarkdown("hello world")).toContain("<p>hello world</p>");
    expect(renderMarkdown("> quoted")).toContain("<blockquote>quoted</blockquote>");
  });
});
