package dev.openakta.aktadocs;

import org.commonmark.node.Document;
import org.commonmark.node.FencedCodeBlock;
import org.commonmark.node.Heading;
import org.commonmark.node.Node;
import org.commonmark.node.Paragraph;
import org.commonmark.node.Text;
import org.commonmark.parser.Parser;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.time.format.DateTimeParseException;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.Date;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;

public final class LintEngine {

    private static final Pattern DOC_ID = Pattern.compile("^[a-z0-9][a-z0-9._-]*$");
    private static final Pattern ISO_DATE =
            Pattern.compile(
                    "^\\d{4}-\\d{2}-\\d{2}(?:T\\d{2}:\\d{2}:\\d{2}(?:\\.\\d+)?(?:Z|[+-]\\d{2}:\\d{2})?)?$");
    private static final Pattern QUESTION_START =
            Pattern.compile("^(how|what|when|where|why|who|which|can|should|does|is|are)\\b", Pattern.CASE_INSENSITIVE);
    private static final Pattern H23 = Pattern.compile("^(#{2,3})\\s+(.+)$");

    private static final List<String> DOC_TYPES =
            List.of(
                    "adr",
                    "business_rule",
                    "feature",
                    "guide",
                    "reference",
                    "explanation",
                    "research",
                    "meta",
                    "changelog",
                    "technical",
                    "other");

    private static final Parser PARSER = Parser.builder().build();

    private LintEngine() {}

    public static LintResult lintFiles(List<Path> paths, AktaConfig cfg, Path cwd) throws IOException {
        List<Diagnostic> diags = new ArrayList<>();
        for (Path abs : paths) {
            Path rel;
            try {
                rel = cwd.relativize(abs.toAbsolutePath().normalize());
            } catch (Exception e) {
                rel = abs.getFileName();
            }
            String relStr = rel.toString().replace('\\', '/');
            if (relStr.isEmpty()) relStr = abs.getFileName().toString();

            String raw;
            try {
                raw = Files.readString(abs);
            } catch (IOException e) {
                continue;
            }

            Frontmatter.Result fm;
            try {
                fm = Frontmatter.parse(raw);
            } catch (IOException e) {
                push(
                        diags,
                        relStr,
                        "META-001",
                        RuleSeverity.mapSeverity(RuleSeverity.effective(cfg, "META-001")),
                        1,
                        1,
                        "Invalid frontmatter YAML.");
                continue;
            }

            Map<String, Object> data = fm.data();
            String body = fm.body();

            lintMeta(relStr, raw, data, cfg, diags);
            if (!Frontmatter.hasBlock(raw)) {
                continue;
            }

            Document doc;
            try {
                doc = (Document) PARSER.parse(body);
            } catch (Exception e) {
                push(
                        diags,
                        relStr,
                        "META-001",
                        RuleSeverity.mapSeverity(RuleSeverity.effective(cfg, "META-001")),
                        1,
                        1,
                        "Markdown body failed to parse.");
                continue;
            }

            int bodyStartLine = Frontmatter.bodyStartLine(raw, body);
            int lineOffset = bodyStartLine - 1;

            lintMetaQuick(relStr, body, data, doc, cfg, diags, lineOffset);
            lintStruct008(relStr, body, data, cfg, diags, lineOffset);
            lintContent001(relStr, body, cfg, diags, lineOffset);
        }

        diags.sort(
                Comparator.comparing((Diagnostic d) -> d.file())
                        .thenComparingInt(Diagnostic::line)
                        .thenComparingInt(Diagnostic::column)
                        .thenComparing(Diagnostic::ruleId));

        int err = (int) diags.stream().filter(d -> "error".equals(d.severity())).count();
        int warn = (int) diags.stream().filter(d -> "warn".equals(d.severity())).count();
        return new LintResult(diags, new LintSummary(err, warn));
    }

    private static void push(
            List<Diagnostic> diags,
            String file,
            String rule,
            String severity,
            int line,
            int col,
            String msg) {
        if (severity == null) return;
        diags.add(new Diagnostic(file, line, col, rule, severity, msg, null, null));
    }

    private static void push(
            List<Diagnostic> diags,
            String file,
            String rule,
            String severity,
            int line,
            int col,
            String msg,
            Integer endLine,
            Integer endCol) {
        if (severity == null) return;
        diags.add(new Diagnostic(file, line, col, rule, severity, msg, endLine, endCol));
    }

    static void lintMeta(
            String file, String fileContent, Map<String, Object> data, AktaConfig cfg, List<Diagnostic> diags) {
        String r1 = RuleSeverity.effective(cfg, "META-001");
        if (!Frontmatter.hasBlock(fileContent)) {
            push(diags, file, "META-001", RuleSeverity.mapSeverity(r1), 1, 1, "Missing YAML frontmatter (expected leading --- block).");
            return;
        }

        String r2 = RuleSeverity.effective(cfg, "META-002");
        Object docId = data.get("doc_id");
        if (!(docId instanceof String) || ((String) docId).isBlank()) {
            push(diags, file, "META-002", RuleSeverity.mapSeverity(r2), 2, 1, "Frontmatter must include non-empty string `doc_id`.");
        } else if (!DOC_ID.matcher(((String) docId).strip()).matches()) {
            push(diags, file, "META-002", RuleSeverity.mapSeverity(r2), 2, 1, "doc_id must match pattern " + DOC_ID + ": got \"" + docId + "\".");
        }

        String r3 = RuleSeverity.effective(cfg, "META-003");
        Object docType = data.get("doc_type");
        if (!(docType instanceof String) || ((String) docType).isBlank()) {
            push(diags, file, "META-003", RuleSeverity.mapSeverity(r3), 2, 1, "Frontmatter must include string `doc_type`.");
        } else if (!DOC_TYPES.contains(((String) docType).strip())) {
            push(diags, file, "META-003", RuleSeverity.mapSeverity(r3), 2, 1, "Invalid doc_type \"" + docType + "\".");
        }

        String r4 = RuleSeverity.effective(cfg, "META-004");
        Object dateVal = data.get("date");
        String dateStr = null;
        if (dateVal instanceof String s && !s.isBlank()) {
            dateStr = s.strip();
        } else if (dateVal instanceof Date d) {
            dateStr = new java.text.SimpleDateFormat("yyyy-MM-dd").format(d);
        } else if (dateVal instanceof LocalDate ld) {
            dateStr = ld.toString();
        }
        if (dateStr == null) {
            push(diags, file, "META-004", RuleSeverity.mapSeverity(r4), 2, 1, "Frontmatter must include ISO8601 `date` (YYYY-MM-DD or full instant).");
        } else if (!ISO_DATE.matcher(dateStr).matches()) {
            try {
                LocalDate.parse(dateStr, DateTimeFormatter.ISO_LOCAL_DATE);
            } catch (DateTimeParseException e) {
                try {
                    java.time.OffsetDateTime.parse(dateStr, DateTimeFormatter.ISO_OFFSET_DATE_TIME);
                } catch (Exception e2) {
                    push(diags, file, "META-004", RuleSeverity.mapSeverity(r4), 2, 1, "date must be ISO8601: got \"" + dateStr + "\".");
                }
            }
        }
    }

    static void lintMetaQuick(
            String file,
            String body,
            Map<String, Object> data,
            Document doc,
            AktaConfig cfg,
            List<Diagnostic> diags,
            int lineOffset) {
        String sev = RuleSeverity.effective(cfg, "META-QUICK");
        String eff = RuleSeverity.mapSeverity(sev);
        if (eff == null) return;
        if ("changelog".equals(String.valueOf(data.get("doc_type")))) return;

        int minW = (int) RuleSeverity.numberOption(cfg, "META-QUICK", "min_words", 40);
        int maxW = (int) RuleSeverity.numberOption(cfg, "META-QUICK", "max_words", 80);

        FencedCodeBlock quick = findQuickFence(doc);
        if (quick == null) {
            push(diags, file, "META-QUICK", eff, 1 + lineOffset, 1, "After the first H1, expected a fenced code block with language tag `quick` (```quick).");
            return;
        }
        String text = quick.getLiteral() == null ? "" : quick.getLiteral().strip();
        int words = WordCount.count(text);
        int lineInBody = fenceQuickOpenLine1(body);
        int line = lineInBody + lineOffset;
        int col = 1;
        if (words < minW || words > maxW) {
            push(diags, file, "META-QUICK", eff, line, col, "Quick Answer block must be " + minW + "–" + maxW + " words; found " + words + ".", null, null);
        }
    }

    /** 1-based line number within body of the line containing ```quick */
    private static int fenceQuickOpenLine1(String body) {
        int idx = body.indexOf("```quick");
        if (idx < 0) idx = body.indexOf("``` quick");
        if (idx < 0) return 1;
        int n = 1;
        for (int i = 0; i < idx; i++) {
            if (body.charAt(i) == '\n') n++;
        }
        return n;
    }

    private static FencedCodeBlock findQuickFence(Document doc) {
        Node n = doc.getFirstChild();
        while (n != null) {
            if (n instanceof Heading h && h.getLevel() == 1) {
                n = n.getNext();
                while (n != null) {
                    if (n instanceof Heading) return null;
                    if (n instanceof FencedCodeBlock fcb) {
                        String info = trimInfo(fcb.getInfo());
                        if ("quick".equals(info)) return fcb;
                        return null;
                    }
                    if (n instanceof Paragraph p) {
                        if (paragraphBlank(p)) {
                            n = n.getNext();
                            continue;
                        }
                        return null;
                    }
                    return null;
                }
                return null;
            }
            n = n.getNext();
        }
        return null;
    }

    private static String trimInfo(String info) {
        if (info == null) return "";
        String[] parts = info.strip().split("\\s+");
        return parts.length > 0 ? parts[0] : "";
    }

    private static boolean paragraphBlank(Paragraph p) {
        Node c = p.getFirstChild();
        if (c == null) return true;
        if (c instanceof Text t) return t.getLiteral() == null || t.getLiteral().isBlank();
        return false;
    }

    static void lintStruct008(
            String file,
            String body,
            Map<String, Object> data,
            AktaConfig cfg,
            List<Diagnostic> diags,
            int lineOffset) {
        String sev = RuleSeverity.effective(cfg, "STRUCT-008");
        String eff = RuleSeverity.mapSeverity(sev);
        if (eff == null) return;
        if ("changelog".equals(String.valueOf(data.get("doc_type")))) return;

        int minW = (int) RuleSeverity.numberOption(cfg, "STRUCT-008", "min_words", 150);
        int maxW = (int) RuleSeverity.numberOption(cfg, "STRUCT-008", "max_words", 300);

        List<HeadingInfo> hs = collectH23(body);
        hs.sort(Comparator.comparingInt(HeadingInfo::line0));

        String[] lines = body.split("\\R", -1);

        for (HeadingInfo h : hs) {
            int firstContent0 = h.line0() + 1;
            HeadingInfo nxt =
                    hs.stream()
                            .filter(
                                    x ->
                                            x != h
                                                    && x.line0() > h.line0()
                                                    && x.depth() <= h.depth())
                            .findFirst()
                            .orElse(null);
            int endExclusive = nxt == null ? lines.length : nxt.line0();
            if (firstContent0 > endExclusive) {
                push(
                        diags,
                        file,
                        "STRUCT-008",
                        eff,
                        h.line0() + 1 + lineOffset,
                        1,
                        "Section has no body; cannot satisfy STRUCT-008 word range.",
                        h.line0() + 1 + lineOffset,
                        null);
                continue;
            }
            StringBuilder sb = new StringBuilder();
            for (int i = firstContent0; i < endExclusive; i++) {
                if (i > firstContent0) sb.append('\n');
                sb.append(lines[i]);
            }
            int wc = WordCount.count(sb.toString());
            int line = h.line0() + 1 + lineOffset;
            if (wc < minW || wc > maxW) {
                push(
                        diags,
                        file,
                        "STRUCT-008",
                        eff,
                        line,
                        1,
                        "H" + h.depth() + " section must be " + minW + "–" + maxW + " words; found " + wc + ".",
                        h.line0() + 1 + lineOffset,
                        null);
            }
        }
    }

    static List<HeadingInfo> collectH23(String body) {
        List<HeadingInfo> out = new ArrayList<>();
        String[] lines = body.split("\\R", -1);
        for (int i = 0; i < lines.length; i++) {
            var m = H23.matcher(lines[i]);
            if (m.matches()) {
                int depth = m.group(1).length();
                if (depth == 2 || depth == 3) {
                    out.add(new HeadingInfo(depth, i, m.group(2).strip()));
                }
            }
        }
        return out;
    }

    static void lintContent001(
            String file, String body, AktaConfig cfg, List<Diagnostic> diags, int lineOffset) {
        String sev = RuleSeverity.effective(cfg, "CONTENT-001");
        String eff = RuleSeverity.mapSeverity(sev);
        if (eff == null) return;
        double ratio = RuleSeverity.numberOption(cfg, "CONTENT-001", "min_question_ratio", 0.7);
        List<HeadingInfo> hs = collectH23(body);
        if (hs.isEmpty()) return;
        int q = 0;
        for (HeadingInfo h : hs) {
            if (isQuestion(h.text())) q++;
        }
        double actual = (double) q / hs.size();
        if (actual + 1e-9 < ratio) {
            push(
                    diags,
                    file,
                    "CONTENT-001",
                    eff,
                    1 + lineOffset,
                    1,
                    String.format(
                            "At least %.0f%% of H2/H3 headings should be questions; got %.1f%% (%d/%d).",
                            ratio * 100, actual * 100, q, hs.size()));
        }
    }

    private static boolean isQuestion(String text) {
        String s = text.strip();
        if (s.endsWith("?")) return true;
        return QUESTION_START.matcher(s).find();
    }
}
