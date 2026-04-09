summon pact/test.{describe, it, expect, mock}
summon {{name}}.{{{name}}_agent}

describe "{{name}}" {
    it "calls echo tool and returns result" {
        fix agent_mock = mock(AgentIO, #{
            call_tool: fun(name, args) { args }
            log:       fun(msg)       { () }
        })

        fix result = with agent_mock {
            {{name}}_agent()
        }

        expect(result) >> to_equal(Json.string("hello"))
    }
}
