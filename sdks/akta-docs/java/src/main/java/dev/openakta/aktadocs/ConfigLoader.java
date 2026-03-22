package dev.openakta.aktadocs;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.dataformat.yaml.YAMLFactory;

import java.io.IOException;
import java.nio.file.Path;

public final class ConfigLoader {

    private static final ObjectMapper YAML = new ObjectMapper(new YAMLFactory());

    private ConfigLoader() {}

    public static AktaConfig load(Path path) throws IOException {
        return YAML.readValue(path.toFile(), AktaConfig.class);
    }
}
