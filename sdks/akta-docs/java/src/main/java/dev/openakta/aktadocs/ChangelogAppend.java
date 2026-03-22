package dev.openakta.aktadocs;

import com.fasterxml.jackson.databind.ObjectMapper;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;

public final class ChangelogAppend {

    public static final String ANCHOR = "<!-- akta-changelog-append -->";

    private static final ObjectMapper JSON = new ObjectMapper();

    private ChangelogAppend() {}

    public record Result(String target, int bytesWritten, boolean created) {}

    public static Result append(Path target, String jsonPayload, String template, boolean dryRun)
            throws IOException {
        ChangelogEntryPayload entry = JSON.readValue(jsonPayload, ChangelogEntryPayload.class);
        String line =
                "- **"
                        + entry.changeType
                        + "** ("
                        + entry.timestamp
                        + ") "
                        + entry.summary;
        if ("detailed".equals(template) && entry.details != null && !entry.details.isBlank()) {
            line += "\n  " + entry.details.replace("\n", "\n  ");
        }
        String block = "\n" + line + "\n";

        String out;
        boolean created;
        if (!Files.exists(target)) {
            created = true;
            out =
                    "---\n"
                            + "doc_id: "
                            + entry.docId
                            + "\n"
                            + "doc_type: changelog\n"
                            + "date: "
                            + entry.timestamp.substring(0, Math.min(10, entry.timestamp.length()))
                            + "\n"
                            + "---\n\n"
                            + "# Changelog\n\n"
                            + ANCHOR
                            + block;
        } else {
            created = false;
            String existing = Files.readString(target);
            if (existing.contains(ANCHOR)) {
                out = existing.replace(ANCHOR, ANCHOR + block);
            } else {
                String sep = existing.endsWith("\n") ? "\n" : "\n\n";
                out = existing + sep + block.strip() + "\n";
            }
        }

        byte[] bytes = out.getBytes(java.nio.charset.StandardCharsets.UTF_8);
        if (dryRun) {
            return new Result(target.toString(), bytes.length, created);
        }
        Files.createDirectories(target.getParent());
        Path tmp = Files.createTempFile(target.getParent(), ".akta-changelog-", ".md");
        Files.write(tmp, bytes);
        Files.move(tmp, target, StandardCopyOption.REPLACE_EXISTING);
        return new Result(target.toString(), bytes.length, created);
    }
}
