export function initConsoleBranding() {
  if (typeof window === 'undefined') return;

  const titleStyle = [
    'color: #00d4aa',
    'font-size: 20px',
    'font-weight: bold',
    'font-family: monospace',
    'text-shadow: 0 0 10px #00d4aa',
  ].join(';');

  const lineStyle = [
    'color: #00d4aa',
    'font-size: 12px',
    'font-family: monospace',
  ].join(';');

  const urlStyle = [
    'color: #6b8aff',
    'font-size: 11px',
    'font-family: monospace',
  ].join(';');

  const chineseStyle = [
    'color: #ffd93d',
    'font-size: 13px',
    'font-family: monospace',
  ].join(';');

  const apiStyle = [
    'color: #ff7b72',
    'font-size: 12px',
    'font-weight: bold',
    'font-family: monospace',
  ].join(';');

  console.log(
    '%cB E N N E T T   S T U D I O\n' +
    '%csilicon swimming ducks isotope foundation\n' +
    '%cThe Database Workspace for Modern Developers\n' +
    '%c林深时见鹿，海深时见鲸，情深时见你 ann\n' +
    '%cCrawling ten thousand miles is no match for taking the API route\n' +
    '%cOne-click access to Bennett Studio local database query\n' +
    '%chttps://bennett.studio',
    titleStyle, lineStyle, lineStyle, chineseStyle, apiStyle, apiStyle, urlStyle
  );
}