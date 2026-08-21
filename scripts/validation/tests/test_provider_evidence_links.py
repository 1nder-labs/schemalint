import io
import sys
import unittest
import urllib.error
from unittest.mock import patch

from scripts.validation import provider_evidence_links as links


class Response:
    def __init__(self, url, body=b""):
        self.url = url
        self.body = body

    def __enter__(self):
        return self

    def __exit__(self, *_args):
        return None

    def geturl(self):
        return self.url

    def read(self):
        if isinstance(self.body, Exception):
            raise self.body
        return self.body


class ProviderEvidenceLinksTests(unittest.TestCase):
    def test_offline_rejects_unknown_hosts_and_missing_fragments(self):
        rows = [
            ("openai.toml", "rule-a", "https://example.com/guide#rule-a"),
            ("openai.toml", "rule-b", "https://developers.openai.com/guide"),
        ]

        errors = links.offline_errors(rows)

        self.assertEqual(2, len(errors))
        self.assertIn("disallowed provider URL", errors[0])
        self.assertIn("has no fragment", errors[1])

    @patch.object(links.urllib.request, "urlopen")
    def test_audit_caches_pages_for_multiple_fragments(self, urlopen):
        urlopen.return_value = Response(
            "https://developers.openai.com/guide",
            b'<h2 id="one"></h2><a name="two"></a>',
        )
        pages = {}

        self.assertEqual(
            ("valid", "ok"),
            links.audit("https://developers.openai.com/guide#one", pages),
        )
        self.assertEqual(
            ("valid", "ok"),
            links.audit("https://developers.openai.com/guide#two", pages),
        )
        urlopen.assert_called_once()

    @patch.object(links.urllib.request, "urlopen")
    def test_audit_reports_redirect_and_missing_fragment(self, urlopen):
        urlopen.side_effect = [
            Response("https://developers.openai.com/new", b'<h2 id="rule"></h2>'),
            Response("https://platform.claude.com/guide", b"<html></html>"),
        ]

        self.assertEqual(
            ("drift", "redirected to https://developers.openai.com/new"),
            links.audit("https://developers.openai.com/old#rule", {}),
        )
        self.assertEqual(
            ("broken", "missing fragment #rule"),
            links.audit("https://platform.claude.com/guide#rule", {}),
        )

    @patch.object(links.time, "sleep")
    @patch.object(links.urllib.request, "urlopen")
    def test_read_errors_are_retried_then_cached_as_inconclusive(self, urlopen, sleep):
        urlopen.side_effect = [
            Response("https://developers.openai.com/guide", OSError("reset")),
            Response("https://developers.openai.com/guide", OSError("reset")),
            Response("https://developers.openai.com/guide", OSError("reset")),
        ]
        pages = {}
        url = "https://developers.openai.com/guide#rule"

        status, detail = links.audit(url, pages)
        cached = links.audit(url, pages)

        self.assertEqual("inconclusive", status)
        self.assertIn("could not read response", detail)
        self.assertEqual((status, detail), cached)
        self.assertEqual(3, urlopen.call_count)
        self.assertEqual(2, sleep.call_count)

    @patch.object(links.Anchors, "feed", side_effect=ValueError("bad html"))
    @patch.object(links.urllib.request, "urlopen")
    def test_parse_errors_are_cached_as_inconclusive(self, urlopen, _feed):
        urlopen.return_value = Response(
            "https://developers.openai.com/guide", b"<html></html>"
        )
        pages = {}
        url = "https://developers.openai.com/guide#rule"

        first = links.audit(url, pages)
        second = links.audit(url, pages)

        self.assertEqual(("inconclusive", "could not parse response: bad html"), first)
        self.assertEqual(first, second)
        urlopen.assert_called_once()

    @patch.object(links.time, "sleep")
    @patch.object(links.urllib.request, "urlopen")
    def test_transient_errors_do_not_abort_later_urls(self, urlopen, _sleep):
        urlopen.side_effect = [
            urllib.error.URLError("offline"),
            urllib.error.URLError("offline"),
            urllib.error.URLError("offline"),
            Response("https://platform.claude.com/guide", b'<h2 id="rule"></h2>'),
        ]
        pages = {}

        first = links.audit("https://developers.openai.com/guide#rule", pages)
        second = links.audit("https://platform.claude.com/guide#rule", pages)

        self.assertEqual("inconclusive", first[0])
        self.assertEqual(("valid", "ok"), second)

    @patch.object(links, "offline_errors", return_value=[])
    @patch.object(links, "evidence_urls")
    @patch.object(links, "audit", return_value=("inconclusive", "offline"))
    def test_network_main_returns_two_when_nothing_is_conclusive(
        self, _audit, evidence_urls, _offline_errors
    ):
        evidence_urls.return_value = [
            ("openai.toml", "rule", "https://developers.openai.com/guide#rule")
        ]

        with patch.object(sys, "argv", ["provider_evidence_links.py", "--network"]), patch(
            "sys.stdout", new_callable=io.StringIO
        ), patch("sys.stderr", new_callable=io.StringIO) as stderr:
            result = links.main()

        self.assertEqual(2, result)
        self.assertIn("conclusively checked", stderr.getvalue())

    @patch.object(links, "offline_errors", return_value=[])
    @patch.object(links, "evidence_urls")
    @patch.object(links, "audit", side_effect=[("broken", "HTTP 404"), ("inconclusive", "offline")])
    def test_network_main_preserves_broken_failure(self, _audit, evidence_urls, _offline_errors):
        evidence_urls.return_value = [
            ("openai.toml", "one", "https://developers.openai.com/guide#one"),
            ("anthropic.toml", "two", "https://platform.claude.com/guide#two"),
        ]

        with patch.object(sys, "argv", ["provider_evidence_links.py", "--network"]), patch(
            "sys.stdout", new_callable=io.StringIO
        ):
            result = links.main()

        self.assertEqual(1, result)


if __name__ == "__main__":
    unittest.main()
