#!/usr/bin/env python3
"""Exercise routing preflight behavior without a running Studio instance."""
import runpy
from pathlib import Path
import unittest

check = runpy.run_path(str(Path(__file__).with_name("check-model-routing")))
evaluate = check["evaluate"]


class RoutingTests(unittest.TestCase):
    def setUp(self):
        self.agents = [{"id": "a", "name": "Agent", "enabled": True, "employment_status": "active"}]
        self.bindings = [{"agent_id": "a", "model_id": "m", "priority": 1}]
        self.models = [{"id": "m", "name": "coder", "provider_id": "p", "enabled": True}]
        self.providers = [{"id": "p", "name": "Host", "enabled": True, "provider_type": "openai_compatible"}]

    def evaluate(self, discover):
        return evaluate(self.agents, self.bindings, self.models, self.providers, discover)

    def test_discovery_pages_and_missing_model(self):
        def discover(_, cursor):
            return {"models": [{"id": "coder"}] if cursor else [{"id": "other"}], "next_cursor": None if cursor else "next"}
        self.assertTrue(self.evaluate(discover)[0][0])
        result = self.evaluate(lambda *_: {"models": [], "next_cursor": None})
        self.assertFalse(result[0][0])
        self.assertIn("not advertised", result[0][1])

    def test_unknown_unbound_disabled_voice_and_ambiguous(self):
        discover = lambda *_: (_ for _ in ()).throw(OSError("private endpoint"))
        result = self.evaluate(discover)
        self.assertIn("unknown", result[0][1])
        self.assertNotIn("private endpoint", result[0][1])
        self.bindings.clear()
        self.assertIn("fallback is not checked", self.evaluate(discover)[0][1])
        self.bindings.extend([{"agent_id": "a", "model_id": "m", "priority": 1}] * 2)
        self.assertIn("same priority", self.evaluate(discover)[0][1])
        self.bindings.pop()
        self.providers[0]["enabled"] = False
        self.assertIn("disabled", self.evaluate(discover)[0][1])
        self.providers[0]["enabled"] = True
        self.models[0]["name"] = " DEEPSEEK-VOICE-TRANSCRIPTION "
        self.assertIn("voice-only", self.evaluate(discover)[0][1])

    def test_repeated_cursor_is_unknown(self):
        result = self.evaluate(lambda *_: {"models": [], "next_cursor": "same"})
        self.assertIn("unknown", result[0][1])


if __name__ == "__main__":
    unittest.main()
