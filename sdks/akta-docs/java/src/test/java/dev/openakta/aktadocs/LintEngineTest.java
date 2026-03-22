package dev.openakta.aktadocs;

import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertTrue;

class LintEngineTest {

    static Path fixtures() {
        return Path.of("..", "typescript", "tests", "fixtures").toAbsolutePath().normalize();
    }

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

    static Path requireFixture(String name) {
        Path fixture = fixtures().resolve(name);
        if (Files.exists(fixture)) {
            return fixture;
        }

        boolean ci = "true".equalsIgnoreCase(System.getenv("CI"));
        boolean skipFixtures = "1".equals(System.getenv("SKIP_FIXTURES"));
        if (ci && !skipFixtures) {
            throw new AssertionError("Missing shared TS fixture: " + fixture);
        }

        Assumptions.assumeTrue(false, () -> "Shared TS fixtures not present: " + fixture);
        return fixture;
    }

    @Test
    void meta001NoFrontmatter() throws Exception {
        Path f = requireFixture("no-frontmatter.md");
        Path cwd = fixtures().getParent();
        LintResult r = LintEngine.lintFiles(List.of(f), strict(), cwd);
        assertTrue(r.diagnostics().stream().anyMatch(d -> "META-001".equals(d.ruleId())));
    }

    @Test
    void compliantPasses() throws Exception {
        Path f = requireFixture("compliant.md");
        Path cwd = fixtures().getParent();
        LintResult r = LintEngine.lintFiles(List.of(f), strict(), cwd);
        long errors = r.diagnostics().stream().filter(d -> "error".equals(d.severity())).count();
        assertTrue(errors == 0);
    }

    @Test
    void struct008ShortSection() throws Exception {
        Path f = requireFixture("short-section.md");
        Path cwd = fixtures().getParent();
        LintResult r = LintEngine.lintFiles(List.of(f), strict(), cwd);
        assertTrue(r.diagnostics().stream().anyMatch(d -> "STRUCT-008".equals(d.ruleId())));
    }

    @Test
    void wordCount() {
        org.junit.jupiter.api.Assertions.assertEquals(3, WordCount.count("a b c"));
    }
}
