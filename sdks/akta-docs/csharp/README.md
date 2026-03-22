# akta-docs (C# / .NET)

Port of **akta-docs** using **System.CommandLine**, **YamlDotNet**, **Markdig**, and **System.Text.Json**, aligned with the TypeScript reference in [`../typescript/`](../typescript/).

## Requirements

- [.NET SDK 7](https://dotnet.microsoft.com/download) or later (project targets `net7.0`).

## Build

```bash
dotnet build AktaDocs.sln
```

## Test

From `sdks/akta-docs/csharp` (fixtures resolve to `../typescript/tests/fixtures/`). The solution includes the app and test projects so **`dotnet test` discovers tests** without passing a `.csproj`:

```bash
dotnet test AktaDocs.sln --configuration Debug
```

## Run

```bash
dotnet run -- --help
```

The executable assembly name is `akta-docs` (see `AktaDocs.csproj`).

## Commands

`init`, `lint`, `create`, `changelog append` — see `--help` and [`../typescript/PORTING.md`](../typescript/PORTING.md) for parity with the reference implementation.

The Rust workspace command `openakta doc lint` currently has a different linter/config surface. Use [`../typescript/`](../typescript/) as the reference when validating parity for this port.
