// Volume search — the home page shows ONLY a status message, it does NOT list the
// records and does NOT open a modal. To read the leaked data (Act 1 SQLi) a player
// must inspect the raw response via DevTools -> Network or curl/Postman. The backend
// keeps the intentional vulnerability: both the results and the SQL error message are
// still fully present in the HTTP response.
async function doSearch() {
  const query = document.getElementById('q').value;
  const out = document.getElementById('out');
  const r = await fetch(`/search?q=${encodeURIComponent(query)}`);
  const data = await r.json();

  out.textContent = buildMessage(data);
}

// Return a plain status message — it never reveals record contents or error details.
function buildMessage(data) {
  if (data && data.error) {
    // The SQL error detail is still in the response (Network tab) for column enumeration.
    return '⚠ The archive ran into a problem while processing the search query.';
  }

  const results = (data && data.results) || [];
  if (results.length === 0) {
    return 'No volumes matched your keyword.';
  }
  return `Found ${results.length} volume(s) in the archive.`;
}
