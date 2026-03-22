# akta-docs (Java)

Java port of **akta-docs**: CLI (`picocli`), config (Jackson YAML/JSON), Markdown linting (**commonmark-java**), aligned with the TypeScript reference under [`../typescript/`](../typescript/).

## Build

```bash
mvn -q compile
```

## Test

Tests load fixtures from `../typescript/tests/fixtures/` (run Maven from `sdks/akta-docs/java`).

```bash
mvn test
```

## Run

```bash
mvn -q dependency:build-classpath -Dmdep.outputFile=cp.txt
java -cp "target/classes:$(cat cp.txt)" dev.openakta.aktadocs.AktaDocsApp --help
```

After `mvn package`, the runnable JAR expects dependencies on the classpath unless you build a shaded/fat JAR (not configured by default).

## Commands

`init`, `lint`, `create`, `changelog append` — see `--help` and [`../typescript/PORTING.md`](../typescript/PORTING.md) for parity with the reference implementation.

Do not assume `openakta doc lint` from the Rust workspace is interchangeable with this CLI. The portable parity target remains [`../typescript/`](../typescript/).
