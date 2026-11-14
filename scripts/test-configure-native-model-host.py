#!/usr/bin/env python3
"""Regression checks for native host restart synchronization."""
import contextlib
import http.server
import importlib.machinery
import io
from pathlib import Path
import sys
import threading
import unittest
import urllib.error
from unittest import mock

loader = importlib.machinery.SourceFileLoader("configure_native_model_host", str(Path(__file__).with_name("configure-native-model-host")))
module = loader.load_module()


class WatchTests(unittest.TestCase):
    def test_discovery_does_not_forward_token_on_redirect(self):
        requests = []
        class Target(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                requests.append(self.headers.get("Authorization"))
                self.send_response(200); self.end_headers()
            def log_message(self, *_): pass
        target = http.server.HTTPServer(("127.0.0.1", 0), Target)
        class Source(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                self.send_response(302)
                self.send_header("Location", f"http://127.0.0.1:{target.server_port}/stolen")
                self.end_headers()
            def log_message(self, *_): pass
        source = http.server.HTTPServer(("127.0.0.1", 0), Source)
        threads = [threading.Thread(target=server.serve_forever, daemon=True) for server in (source, target)]
        for thread in threads: thread.start()
        try:
            with mock.patch.object(module, "mcp") as mcp:
                with self.assertRaises(urllib.error.HTTPError):
                    module.configure(source.server_port, "synthetic-token", "coding", "Fixture")
                mcp.assert_not_called()
            self.assertEqual(requests, [])
        finally:
            for server in (source, target): server.shutdown(); server.server_close()
            for thread in threads: thread.join()

    def test_known_configuration_error_is_actionable_without_exposing_token(self):
        with mock.patch.object(sys, "argv", ["configure-native-model-host", ".", "--model", "fixture"]), \
             mock.patch.object(module, "read_host", return_value=(10001, "synthetic-token")), \
             mock.patch.object(module, "configure", side_effect=module.SetupError("Requested model is not an installed text model on the selected host")), \
             contextlib.redirect_stderr(io.StringIO()) as errors:
            self.assertEqual(module.main(), 1)
        self.assertIn("not an installed text model", errors.getvalue())
        self.assertNotIn("synthetic-token", errors.getvalue())

    def test_refresh_restores_host_completion_profile_and_rejects_other_urls(self):
        existing = {"id": "provider", "name": "Fixture", "provider_type": "openai_compatible", "protocol": "openai_compatible", "base_url": "http://host.docker.internal:10001/v1", "completion_profile": "openai_reasoning"}
        def action(name, args):
            if name == "list_providers": return [existing]
            if name == "update_provider": return {"id": "provider", "name": "Fixture"}
            if name == "discover_provider_models": return {"models": [{"id": "coding"}]}
            if name == "list_model_definitions": return [{"provider_id": "provider", "name": "coding"}]
            raise AssertionError(name)
        response = mock.MagicMock()
        response.__enter__.return_value.read.return_value = b'{"data":[{"id":"coding","task":"text"}]}'
        with mock.patch.object(module.urllib.request, "build_opener") as opener, mock.patch.object(module, "mcp", side_effect=action) as mcp:
            opener.return_value.open.return_value = response
            module.configure(10002, "synthetic-token", "coding", "Fixture")
            update = next(call.args[1] for call in mcp.call_args_list if call.args[0] == "update_provider")
            self.assertEqual(update["completion_profile"], "standard")
            self.assertEqual(update["base_url"], "http://host.docker.internal:10002/v1")
            for address in ["http://host.docker.internal.evil:10001/v1", "http://host.docker.internal:10001/other", "http://user@host.docker.internal:10001/v1", "http://host.docker.internal:10001/v1?secret=1"]:
                existing["base_url"] = address
                mcp.reset_mock()
                with self.assertRaises(ValueError):
                    module.configure(10002, "synthetic-token", "coding", "Fixture")
                self.assertFalse(any(call.args[0] == "update_provider" for call in mcp.call_args_list))
            response.__enter__.return_value.read.return_value = b'{"data":[{"id":"coding","task":"speech"}]}'
            mcp.reset_mock()
            with self.assertRaises(ValueError):
                module.configure(10002, "synthetic-token", "coding", "Fixture")
            mcp.assert_not_called()

    def test_retries_outage_and_updates_only_when_connection_changes(self):
        states = [(10001, "synthetic-one"), (10001, "synthetic-one"), OSError("synthetic-secret"), (10002, "synthetic-two")]
        def state(*_):
            value = states.pop(0)
            if isinstance(value, Exception):
                raise value
            return value
        def stop_after_four_sleeps(_):
            if not states:
                raise KeyboardInterrupt()
        result = {"provider": "Fixture", "base_url": "http://host.docker.internal:10001/v1", "model": "coding"}
        with mock.patch.object(sys, "argv", ["configure-native-model-host", ".", "--watch", "--poll-seconds", "1"]), \
             mock.patch.object(module, "read_host", side_effect=state), \
             mock.patch.object(module, "configure", return_value=result) as configure, \
             mock.patch.object(module.time, "sleep", side_effect=stop_after_four_sleeps), \
             contextlib.redirect_stdout(io.StringIO()) as output, contextlib.redirect_stderr(io.StringIO()) as errors:
            with self.assertRaises(KeyboardInterrupt):
                module.main()
        self.assertEqual(configure.call_count, 2)
        self.assertEqual([call.args[:2] for call in configure.call_args_list], [(10001, "synthetic-one"), (10002, "synthetic-two")])
        self.assertIn("retrying", errors.getvalue())
        self.assertNotIn("synthetic-secret", errors.getvalue() + output.getvalue())
        self.assertNotIn("synthetic-one", errors.getvalue() + output.getvalue())
        self.assertNotIn("synthetic-two", errors.getvalue() + output.getvalue())

    def test_failed_refresh_is_retried_even_if_endpoint_is_unchanged(self):
        state = (10002, "synthetic-token")
        with mock.patch.object(sys, "argv", ["configure-native-model-host", ".", "--watch", "--poll-seconds", "1"]), \
             mock.patch.object(module, "read_host", return_value=state), \
             mock.patch.object(module, "configure", side_effect=[OSError("private"), {"provider": "Fixture"}]) as configure, \
             mock.patch.object(module.time, "sleep", side_effect=[None, KeyboardInterrupt()]), \
             contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(KeyboardInterrupt):
                module.main()
        self.assertEqual(configure.call_count, 2)


if __name__ == "__main__":
    unittest.main()
