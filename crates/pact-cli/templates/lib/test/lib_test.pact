summon pact/test.{describe, it, expect}
summon {{name}}.{hello}

describe "{{name}}" {
    it "returns greeting" {
        expect(hello()) >> to_equal("hello from {{name}}")
    }
}
