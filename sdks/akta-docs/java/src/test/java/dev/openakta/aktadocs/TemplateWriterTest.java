package dev.openakta.aktadocs;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;

class TemplateWriterTest {

    static AktaConfig strict() {
        AktaConfig cfg = new AktaConfig();
        cfg.schemaVersion = "1.0.0";
        cfg.project = new ProjectCfg();
        cfg.project.name = "Test";
        cfg.project.slug = "test";
        cfg.paths = new PathsCfg();
        cfg.paths.docsRoot = "./akta-docs";
        cfg.paths.includeGlobs = List.of("**/*.md");
        cfg.paths.excludeGlobs = List.of();
        cfg.linter = new LinterCfg();
        cfg.scaffold = new ScaffoldCfg();
        cfg.changelog = new ChangelogCfg();
        return cfg;
    }

    @Test
    void adrPassesLint(@TempDir Path dir) throws Exception {
        Path out = dir.resolve("adr.md");
        TemplateWriter.write(
                "adr", out, "Use Postgres", "postgres", "test.postgres.adr", "2025-03-21");
        LintResult r = LintEngine.lintFiles(List.of(out), strict(), dir);
        long errors = r.diagnostics().stream().filter(d -> "error".equals(d.severity())).count();
        assertEquals(0, errors);
    }
}
