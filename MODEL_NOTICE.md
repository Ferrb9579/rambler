# Model notice

This application loads local model weights from `assets/models/`, so
redistribution must follow each model's license and terms.

- **S1-mini by Superwhisper**: [model card and license](https://huggingface.co/superwhisper/s1-mini). The model's naming requirement applies when redistributing it.
- **Whisper base.en Q5_1 (default)**: [whisper.cpp model files and MIT license](https://huggingface.co/ggerganov/whisper.cpp).
- **Whisper large-v3-turbo Q5_0 (optional)**: [model card and MIT license](https://huggingface.co/openai/whisper-large-v3-turbo).
- **GGML Whisper conversion**: [whisper.cpp model files](https://huggingface.co/ggerganov/whisper.cpp).

Downloaded asset checksums (SHA-256):

```text
4baf70dd0d7c4247ba2b81fafd9c01005ac77c2f9ef064e00dcf195d0e2fdd2f  ggml-base.en-q5_1.bin
394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2  ggml-large-v3-turbo-q5_0.bin
3b41ebe2502cbd03e811d5d16b022f5ab551eda58d62597d152f89535003c634  s1-mini-q4_k_m.gguf
aeb13307a71acd8fe81861d94ad54ab689df773318809eed3cbe794b4492dae4  s1-mini-tokenizer.json
```
