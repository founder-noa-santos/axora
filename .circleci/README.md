# CircleCI

Modo econômico:
- nenhuma validação pesada foi colocada para rodar automaticamente
- escolha exatamente uma `action` ao disparar a pipeline manualmente
- jobs caros ficaram separados e opcionais

Ações disponíveis:
- `quality`
- `msrv`
- `advisories`
- `proto`
- `slow`
- `packaged-linux`
- `coverage`
- `mutants`

Notas operacionais:
- `quality` é a melhor rotina padrão
- `slow` roda `#[ignore]` e por isso custa mais
- `packaged-linux` foi mantido só em Linux por economia; macOS ficou fora do CircleCI por custo
- `proto` usa `buf breaking` contra `main`
