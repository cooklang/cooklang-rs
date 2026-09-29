// Cooklang highlighting for the Ace editor (loaded from the CDN as window.ace).
// Token names become CSS classes: ingredient -> .ace_ingredient, etc.

const NAME = "[^\\s@#~{}.,;:!?()\\[\\]]+";
// Multi-word names need braces: @plain flour{125%g}. The class excludes the
// markers so a lone "@salt and @pepper{}" falls back to the one-word form.
const MULTI = "[^@#~{}\\n]+?\\{[^}\\n]*\\}";
const MODIFIERS = "[&?\\-+=]*";
const AFTER = "(?:\\([^)\\n]*\\))?"; // preparation note, e.g. (diced)

export function defineCooklangMode(): void {
    const ace = window.ace;
    ace.define(
        "ace/mode/cooklang_highlight_rules",
        ["require", "exports", "module", "ace/lib/oop", "ace/mode/text_highlight_rules"],
        function (require: any, exports: any) {
            const oop = require("ace/lib/oop");
            const TextHighlightRules = require("ace/mode/text_highlight_rules").TextHighlightRules;

            const component = (marker: string, token: string) => [
                { token, regex: `\\${marker}${MODIFIERS}(?:${MULTI}|${NAME}(?:\\{[^}\\n]*\\})?)${AFTER}` },
            ];

            const CooklangHighlightRules = function (this: any) {
                this.$rules = {
                    start: [
                        { token: "frontmatter", regex: "^---\\s*$", next: "frontmatter" },
                        { token: "comment", regex: "\\[-", next: "blockcomment" },
                        { token: "comment", regex: "--.*$" },
                        { token: "section", regex: "^\\s*=+.*$" },
                        { token: "note", regex: "^\\s*>.*$" },
                        ...component("@", "ingredient"),
                        ...component("#", "cookware"),
                        { token: "timer", regex: `~(?:${MULTI}|${NAME}\\{[^}\\n]*\\}|\\{[^}\\n]*\\})` },
                        { defaultToken: "text" },
                    ],
                    frontmatter: [
                        { token: "frontmatter", regex: "^---\\s*$", next: "start" },
                        { token: "frontmatter.key", regex: "^[\\w-]+(?=\\s*:)" },
                        { defaultToken: "frontmatter" },
                    ],
                    blockcomment: [
                        { token: "comment", regex: "-\\]", next: "start" },
                        { defaultToken: "comment" },
                    ],
                };
                this.normalizeRules();
            };
            oop.inherits(CooklangHighlightRules, TextHighlightRules);
            exports.CooklangHighlightRules = CooklangHighlightRules;
        }
    );

    ace.define(
        "ace/mode/cooklang",
        ["require", "exports", "module", "ace/lib/oop", "ace/mode/text", "ace/mode/cooklang_highlight_rules"],
        function (require: any, exports: any) {
            const oop = require("ace/lib/oop");
            const TextMode = require("ace/mode/text").Mode;
            const Rules = require("ace/mode/cooklang_highlight_rules").CooklangHighlightRules;
            const Mode = function (this: any) {
                this.HighlightRules = Rules;
            };
            oop.inherits(Mode, TextMode);
            exports.Mode = Mode;
        }
    );
}
