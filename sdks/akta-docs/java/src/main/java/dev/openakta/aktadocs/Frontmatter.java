package dev.openakta.aktadocs;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.dataformat.yaml.YAMLFactory;

import java.io.IOException;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/** Split YAML frontmatter (--- delimited) from markdown body. */
public final class Frontmatter {

    private static final ObjectMapper YAML = new ObjectMapper(new YAMLFactory());
    private static final Pattern BLOCK =
            Pattern.compile("^---\\R(.*?)\\R---\\R", Pattern.DOTALL);

    private Frontmatter() {}

    public record Result(Map<String, Object> data, String body) {}

    public static boolean hasBlock(String raw) {
        return raw.stripLeading().startsWith("---");
    }

    public static Result parse(String raw) throws IOException {
        Matcher m = BLOCK.matcher(raw);
        if (!m.find()) {
            return new Result(Collections.emptyMap(), raw);
        }
        String yamlBlock = m.group(1).strip();
        String body = raw.substring(m.end());
        Map<String, Object> map;
        if (yamlBlock.isEmpty()) {
            map = new LinkedHashMap<>();
        } else {
            map = YAML.readValue(yamlBlock, new TypeReference<>() {});
        }
        return new Result(map, body);
    }

    public static int bodyStartLine(String fileContent, String body) {
        int idx = fileContent.indexOf(body);
        if (idx < 0) return 1;
        int n = 0;
        for (int i = 0; i < idx; i++) {
            if (fileContent.charAt(i) == '\n') n++;
        }
        return n + 1;
    }
}
