package dev.openakta.aktadocs;

import java.io.IOException;
import java.nio.file.FileSystems;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.PathMatcher;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.stream.Stream;

public final class GlobPaths {
    private GlobPaths() {}

    public static List<Path> expandLintPaths(AktaConfig cfg, Path cwd) throws IOException {
        Path root = cwd.resolve(cfg.paths.docsRoot).normalize();
        if (!Files.isDirectory(root)) {
            return List.of();
        }
        List<PathMatcher> includes = new ArrayList<>();
        for (String g : cfg.paths.includeGlobs) {
            includes.add(FileSystems.getDefault().getPathMatcher("glob:" + g));
        }
        List<PathMatcher> excludes = new ArrayList<>();
        for (String ex : cfg.paths.excludeGlobs) {
            excludes.add(FileSystems.getDefault().getPathMatcher("glob:" + ex));
        }
        List<Path> out = new ArrayList<>();
        try (Stream<Path> s = Files.walk(root)) {
            s.filter(Files::isRegularFile)
                    .filter(p -> p.toString().endsWith(".md"))
                    .forEach(
                            p -> {
                                Path rel = root.relativize(p);
                                boolean inc = false;
                                for (PathMatcher m : includes) {
                                    if (m.matches(rel)) {
                                        inc = true;
                                        break;
                                    }
                                }
                                if (!inc) return;
                                for (PathMatcher ex : excludes) {
                                    if (ex.matches(rel)) return;
                                }
                                out.add(p);
                            });
        }
        out.sort(Comparator.comparing(Path::toString));
        return out;
    }
}
