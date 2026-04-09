summon pact/agent.{AgentIO, seal, Policy}
summon pact/json.{Json}

-- {{name}}: a sandboxed agent that runs within an AgentIO seal.
fun {{name}}_agent() => Json with {AgentIO} {
    AgentIO.log("starting {{name}}")
    fix result = AgentIO.call_tool("echo", Json.string("hello"))
    AgentIO.log("done")
    result
}

fun main() {
    fix policy = Policy.new()
        >> Policy.allow_tools(["echo"])

    fix result = seal(policy) {
        {{name}}_agent()
    }

    when result {
        Ok(v)  then AgentIO.log("result: ${v}")
        Err(e) then AgentIO.log("breach: ${e}")
    }
}
