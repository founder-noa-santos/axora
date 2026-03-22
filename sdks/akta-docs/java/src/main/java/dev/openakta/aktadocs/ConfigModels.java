package dev.openakta.aktadocs;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonProperty;

import java.util.HashMap;
import java.util.List;
import java.util.Map;

@JsonIgnoreProperties(ignoreUnknown = true)
class AktaConfig {
    @JsonProperty("schema_version")
    public String schemaVersion;
    public ProjectCfg project;
    public PathsCfg paths;
    public LinterCfg linter = new LinterCfg();
    public ScaffoldCfg scaffold = new ScaffoldCfg();
    public ChangelogCfg changelog = new ChangelogCfg();
}

@JsonIgnoreProperties(ignoreUnknown = true)
class ProjectCfg {
    public String name;
    public String slug;
}

@JsonIgnoreProperties(ignoreUnknown = true)
class PathsCfg {
    @JsonProperty("docs_root")
    public String docsRoot;
    @JsonProperty("include_globs")
    public List<String> includeGlobs = List.of("**/*.md");
    @JsonProperty("exclude_globs")
    public List<String> excludeGlobs =
            List.of("**/node_modules/**", "**/.git/**", "**/99-archive/**");
}

@JsonIgnoreProperties(ignoreUnknown = true)
class LinterCfg {
    @JsonProperty("default_severity")
    public String defaultSeverity = "error";
    public Map<String, RuleOptions> rules = new HashMap<>();
}

@JsonIgnoreProperties(ignoreUnknown = true)
class RuleOptions {
    public String severity;
    @JsonProperty("min_words")
    public Integer minWords;
    @JsonProperty("max_words")
    public Integer maxWords;
    @JsonProperty("min_question_ratio")
    public Double minQuestionRatio;
}

@JsonIgnoreProperties(ignoreUnknown = true)
class ScaffoldCfg {
    @JsonProperty("create_readme_in_each_folder")
    public boolean createReadmeInEachFolder = true;
    public boolean gitkeep;
}

@JsonIgnoreProperties(ignoreUnknown = true)
class ChangelogCfg {
    @JsonProperty("default_target")
    public String defaultTarget;
    @JsonProperty("entry_template")
    public String entryTemplate = "compact";
    @JsonProperty("summary_max_length")
    public int summaryMaxLength = 200;
}

@JsonIgnoreProperties(ignoreUnknown = true)
class ChangelogEntryPayload {
    @JsonProperty("schema_version")
    public String schemaVersion;
    @JsonProperty("doc_id")
    public String docId;
    public String timestamp;
    @JsonProperty("change_type")
    public String changeType;
    public String summary;
    public String details;
    public String scope;
    public List<String> refs;
}
