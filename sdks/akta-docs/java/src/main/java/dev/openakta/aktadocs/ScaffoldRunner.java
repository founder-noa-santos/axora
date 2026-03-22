package dev.openakta.aktadocs;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.LocalDate;
import java.util.ArrayList;
import java.util.List;
import java.util.regex.Pattern;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.dataformat.yaml.YAMLFactory;

public final class ScaffoldRunner {

    private static final ObjectMapper YAML_OUT = new ObjectMapper(new YAMLFactory());

    private static final List<String> SECTION_DIRS =
            List.of(
                    "00-meta",
                    "01-adrs",
                    "02-business-core",
                    "03-business-logic",
                    "04-research",
                    "05-features",
                    "06-technical",
                    "07-guides",
                    "08-references",
                    "09-explanations",
                    "10-changelog",
                    "99-archive");

    private static final Pattern NON_SLUG = Pattern.compile("[^a-z0-9]+");

    private ScaffoldRunner() {}

    public static String slugify(String name) {
        String s = NON_SLUG.matcher(name.toLowerCase()).replaceAll("-").replaceAll("^-|-$", "");
        return s.isEmpty() ? "project" : s;
    }

    public static AktaConfig defaultConfig(String projectName) {
        String slug = slugify(projectName);
        AktaConfig cfg = new AktaConfig();
        cfg.schemaVersion = "1.0.0";
        cfg.project = new ProjectCfg();
        cfg.project.name = projectName;
        cfg.project.slug = slug;
        cfg.paths = new PathsCfg();
        cfg.paths.docsRoot = "./akta-docs";
        cfg.linter = new LinterCfg();
        cfg.linter.rules.put("META-QUICK", ruleOff());
        cfg.linter.rules.put("STRUCT-008", ruleOff());
        cfg.linter.rules.put("CONTENT-001", ruleOff());
        cfg.changelog = new ChangelogCfg();
        cfg.changelog.defaultTarget = "akta-docs/10-changelog/CHANGELOG.md";
        return cfg;
    }

    private static RuleOptions ruleOff() {
        RuleOptions r = new RuleOptions();
        r.severity = "off";
        return r;
    }

    public record Report(String root, String docsRoot, List<String> createdPaths, String configPath) {}

    public static Report run(
            Path root,
            String projectName,
            boolean force,
            boolean dryRun,
            boolean createReadmeInEachFolder,
            boolean gitkeep)
            throws IOException {
        Path rootP = root.toAbsolutePath().normalize();
        Path configPath = rootP.resolve(".akta-config.yaml");
        AktaConfig cfg = defaultConfig(projectName);
        cfg.scaffold = new ScaffoldCfg();
        cfg.scaffold.createReadmeInEachFolder = createReadmeInEachFolder;
        cfg.scaffold.gitkeep = gitkeep;

        Path docsRoot = rootP.resolve("akta-docs");
        List<String> created = new ArrayList<>();

        if (!dryRun && Files.exists(configPath) && !force) {
            throw new IOException(configPath + " already exists; use --force to overwrite.");
        }

        String slug = cfg.project.slug != null ? cfg.project.slug : slugify(projectName);
        String today = LocalDate.now().toString();

        for (String dir : SECTION_DIRS) {
            Path full = docsRoot.resolve(dir);
            if (!dryRun) {
                Files.createDirectories(full);
            }
            created.add("akta-docs/" + dir);

            if (createReadmeInEachFolder) {
                String docId = slug + "." + NON_SLUG.matcher(dir).replaceAll("-") + "-readme";
                String body =
                        "---\n"
                                + "doc_id: "
                                + docId
                                + "\n"
                                + "doc_type: meta\n"
                                + "date: "
                                + today
                                + "\n"
                                + "---\n\n"
                                + "# "
                                + dir
                                + "\n\n"
                                + "Placeholder content for this section. Replace with architecture and business documentation aligned with OPENAKTA GEO standards.\n";
                if (!dryRun) {
                    Files.writeString(full.resolve("README.md"), body);
                }
                created.add("akta-docs/" + dir + "/README.md");
            }
            if (gitkeep && !dryRun) {
                Files.writeString(full.resolve(".gitkeep"), "");
            }
        }

        Path changelogPath = docsRoot.resolve("10-changelog").resolve("CHANGELOG.md");
        String initial =
                "---\n"
                        + "doc_id: "
                        + slug
                        + ".changelog\n"
                        + "doc_type: changelog\n"
                        + "date: "
                        + today
                        + "\n"
                        + "---\n\n"
                        + "# Changelog\n\n"
                        + "<!-- akta-changelog-append -->\n";
        if (!dryRun) {
            Files.createDirectories(changelogPath.getParent());
            Files.writeString(changelogPath, initial);
        }
        created.add("akta-docs/10-changelog/CHANGELOG.md");

        if (!dryRun) {
            String yaml = YAML_OUT.writeValueAsString(cfg);
            Files.writeString(configPath, yaml);
        }

        return new Report(
                rootP.toString(), docsRoot.toString(), created, configPath.toString());
    }

}
