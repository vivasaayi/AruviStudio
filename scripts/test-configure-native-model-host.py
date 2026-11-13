#!/usr/bin/env python3
"""Regression checks for native host restart synchronization."""
import contextlib
import importlib.machinery
import io
from pathlib import Path
import sys
import unittest
from unittest import mock

loader = importlib.machinery.SourceFileLoader("configure_native_model_host", str(Path(__file__).with_name("configure-native-model-host")))
module = loader.load_module()


class WatchTests(unittest.TestCase):
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
