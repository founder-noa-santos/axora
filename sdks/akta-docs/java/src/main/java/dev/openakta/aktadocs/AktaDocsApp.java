package dev.openakta.aktadocs;

import com.fasterxml.jackson.databind.ObjectMapper;

import picocli.CommandLine;
import picocli.CommandLine.Command;
import picocli.CommandLine.Option;
import picocli.CommandLine.Parameters;
import picocli.CommandLine.ParentCommand;
import picocli.CommandLine.ScopeType;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.LocalDate;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.concurrent.Callable;
import java.util.regex.Pattern;
import java.util.stream.Stream;

@Command(
        name = "akta-docs",
        mixinStandardHelpOptions = true,
        version = "akta-docs 0.1.0",
        description = "OPENAKTA documentation linter and scaffolding (GEO / AI context)",
        subcommands = {
            AktaDocsApp.InitCommand.class,
            AktaDocsApp.LintCommand.class,
            AktaDocsApp.CreateCommand.class,
            AktaDocsApp.ChangelogParent.class
        },
        scope = ScopeType.INHERIT)
public class AktaDocsApp implements Callable<Integer> {

    @Option(
            names = {"-c", "--config"},
            description = "Path to .akta-config.yaml",
            scope = ScopeType.INHERIT)
    Path configFile;

    @Option(
            names = "--format",
            defaultValue = "default",
            description = "Lint output: default or json",
            scope = ScopeType.INHERIT)
    String format;

    @Override
    public Integer call() {
        CommandLine.usage(this, System.out);
        return 0;
    }

    public static void main(String[] args) {
        int exit =
                new CommandLine(new AktaDocsApp())
                        .setExecutionExceptionHandler(
                                (ex, cmd, pr) -> {
                                    cmd.getErr().println(ex.getMessage());
                                    return 2;
                                })
                        .execute(args);
        System.exit(exit);
    }

    Path resolveConfig(Path cwd) {
        if (configFile != null) return configFile.toAbsolutePath().normalize();
        return cwd.resolve(".akta-config.yaml");
    }

    @Command(name = "init", description = "Scaffold akta-docs layout and .akta-config.yaml")
    static class InitCommand implements Callable<Integer> {
        @ParentCommand AktaDocsApp root;

        @Option(names = "--root", defaultValue = ".", description = "Repository root")
        Path repoRoot;

        @Option(names = "--force", description = "Overwrite existing config")
        boolean force;

        @Option(names = "--dry-run", description = "Do not write files")
        boolean dryRun;

        @Option(names = "--project-name", description = "Project display name")
        String projectName;

        @Option(names = "--skip-readme", description = "Do not write README in each folder")
        boolean skipReadme;

        @Option(names = "--gitkeep", description = "Write .gitkeep in each folder")
        boolean gitkeep;

        @Override
        public Integer call() throws Exception {
            Path r = repoRoot.toAbsolutePath().normalize();
            String name =
                    projectName != null && !projectName.isBlank()
                            ? projectName
                            : (r.getFileName() != null ? r.getFileName().toString() : "openakta-project");
            ScaffoldRunner.Report rep =
                    ScaffoldRunner.run(
                            r, name, force, dryRun, !skipReadme, gitkeep);
            System.out.println("Created docs tree under " + rep.docsRoot());
            System.out.println("Wrote " + rep.configPath());
            return 0;
        }
    }

    @Command(name = "lint", description = "Lint markdown files")
    static class LintCommand implements Callable<Integer> {
        @ParentCommand AktaDocsApp root;

        @Parameters(paramLabel = "PATHS", arity = "0..*", description = "Files or directories")
        List<Path> paths;

        @Option(names = "--max-warnings", defaultValue = "-1", description = "-1 = unlimited")
        int maxWarnings;

        @Option(names = "--rule", description = "Only these rules (repeatable)")
        List<String> rules;

        @Override
        public Integer call() throws Exception {
            Path cwd = Path.of("").toAbsolutePath();
            Path cfgPath = root.resolveConfig(cwd);
            AktaConfig cfg = ConfigLoader.load(cfgPath);
            List<Path> files;
            if (paths != null && !paths.isEmpty()) {
                files = expandLintInputs(paths);
            } else {
                files = GlobPaths.expandLintPaths(cfg, cwd);
            }
            LintResult res = LintEngine.lintFiles(files, cfg, cwd);
            List<Diagnostic> diags = new ArrayList<>(res.diagnostics());
            if (rules != null && !rules.isEmpty()) {
                diags.removeIf(d -> !rules.contains(d.ruleId()));
            }
            if ("json".equals(root.format)) {
                ObjectMapper om = new ObjectMapper();
                System.out.println(om.writerWithDefaultPrettyPrinter().writeValueAsString(diags));
            } else {
                for (Diagnostic d : diags) {
                    System.out.println(formatDefault(d, cwd));
                }
            }
            long errors = diags.stream().filter(d -> "error".equals(d.severity())).count();
            long warnings = diags.stream().filter(d -> "warn".equals(d.severity())).count();
            long cap = maxWarnings < 0 ? Long.MAX_VALUE : maxWarnings;
            if (errors > 0 || warnings > cap) {
                return 1;
            }
            return 0;
        }

        private static String formatDefault(Diagnostic d, Path cwd) {
            Path p = Path.of(d.file());
            String rel;
            try {
                rel = cwd.relativize(p.toAbsolutePath().normalize()).toString().replace('\\', '/');
            } catch (Exception e) {
                rel = d.file();
            }
            return rel + ":" + d.line() + ":" + d.column() + " " + d.severity() + " " + d.ruleId() + " " + d.message();
        }

        private static List<Path> expandLintInputs(List<Path> paths) throws IOException {
            List<Path> out = new ArrayList<>();
            for (Path p : paths) {
                Path abs = p.toAbsolutePath().normalize();
                if (Files.isDirectory(abs)) {
                    try (Stream<Path> s = Files.walk(abs)) {
                        s.filter(Files::isRegularFile)
                                .filter(x -> x.toString().endsWith(".md"))
                                .filter(x -> !x.toString().contains("node_modules"))
                                .forEach(out::add);
                    }
                } else {
                    out.add(abs);
                }
            }
            out.sort(Comparator.comparing(Path::toString));
            return out.stream().distinct().toList();
        }
    }

    @Command(name = "create", description = "Create a linter-friendly template")
    static class CreateCommand implements Callable<Integer> {
        @ParentCommand AktaDocsApp root;

        @Parameters(index = "0", paramLabel = "KIND") String kind;

        @Parameters(index = "1", paramLabel = "OUTPUT") Path outputPath;

        @Option(names = "--title", required = true) String title;

        @Option(names = "--slug", required = true) String slug;

        @Option(names = "--doc-id") String docId;

        @Override
        public Integer call() throws Exception {
            Path cwd = Path.of("").toAbsolutePath();
            String date = LocalDate.now().toString();
            String did =
                    docId != null && !docId.isBlank()
                            ? docId
                            : reSlug(slug) + "." + date;
            TemplateWriter.write(kind, cwd.resolve(outputPath), title, slug, did, date);
            System.out.println("Wrote " + outputPath);
            return 0;
        }

        private static String reSlug(String slug) {
            return Pattern.compile("[^a-z0-9]+")
                    .matcher(slug.toLowerCase())
                    .replaceAll("-")
                    .replaceAll("^-|-$", "");
        }
    }

    @Command(
            name = "changelog",
            description = "Append and manage changelog entries",
            subcommands = ChangelogAppendCmd.class)
    static class ChangelogParent implements Callable<Integer> {
        @ParentCommand AktaDocsApp root;

        @Override
        public Integer call() {
            return 0;
        }
    }

    @Command(name = "append", description = "Append changelog entry from JSON")
    static class ChangelogAppendCmd implements Callable<Integer> {
        @ParentCommand ChangelogParent parent;

        @Option(names = "--file", required = true) Path file;

        @Option(names = "--payload", description = "JSON file (else stdin)") Path payload;

        @Option(names = "--dry-run") boolean dryRun;

        @Override
        public Integer call() throws Exception {
            Path cwd = Path.of("").toAbsolutePath();
            AktaDocsApp app = parent.root;
            Path cfgPath = app.resolveConfig(cwd);
            String template = "compact";
            try {
                AktaConfig cfg = ConfigLoader.load(cfgPath);
                template = cfg.changelog.entryTemplate;
            } catch (IOException ignored) {
            }
            String raw;
            if (payload != null) {
                raw = Files.readString(payload);
            } else {
                raw = new String(System.in.readAllBytes(), java.nio.charset.StandardCharsets.UTF_8);
            }
            ChangelogAppend.Result r =
                    ChangelogAppend.append(file.toAbsolutePath(), raw, template, dryRun);
            String pfx = dryRun ? "[dry-run] " : "";
            System.out.println(
                    pfx + "Wrote " + r.bytesWritten() + " bytes to " + r.target()
                            + (r.created() ? " (created)" : ""));
            return 0;
        }
    }
}
