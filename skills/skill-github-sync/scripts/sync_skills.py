import os
import sys
import json
import subprocess
import urllib.request
import urllib.error

def get_github_token():
    token = os.environ.get('GITHUB_PERSONAL_ACCESS_TOKEN') or os.environ.get('GITHUB_TOKEN')
    if token:
        return token
    mcp_path = os.path.expanduser(r'~/.gemini/config/mcp_config.json')
    if os.path.exists(mcp_path):
        try:
            with open(mcp_path, 'r', encoding='utf-8') as f:
                data = json.load(f)
                return data.get('mcpServers', {}).get('github', {}).get('env', {}).get('GITHUB_PERSONAL_ACCESS_TOKEN')
        except Exception:
            pass
    return None

def ensure_github_repo(token, repo_name='antigravity-skills'):
    req = urllib.request.Request('https://api.github.com/user', headers={
        'Authorization': f'token {token}',
        'User-Agent': 'Antigravity-Sync'
    })
    with urllib.request.urlopen(req) as resp:
        user_info = json.loads(resp.read().decode('utf-8'))
        username = user_info['login']

    check_req = urllib.request.Request(f'https://api.github.com/repos/{username}/{repo_name}', headers={
        'Authorization': f'token {token}',
        'User-Agent': 'Antigravity-Sync'
    })
    try:
        with urllib.request.urlopen(check_req) as resp:
            print(f'Repository {username}/{repo_name} exists.')
            return username, repo_name
    except urllib.error.HTTPError as e:
        if e.code == 404:
            create_payload = json.dumps({
                'name': repo_name,
                'description': 'Persistent, self-evolved skills library for Antigravity Agent OS.',
                'private': False,
                'auto_init': False
            }).encode('utf-8')
            create_req = urllib.request.Request('https://api.github.com/user/repos', data=create_payload, headers={
                'Authorization': f'token {token}',
                'User-Agent': 'Antigravity-Sync',
                'Content-Type': 'application/json'
            })
            with urllib.request.urlopen(create_req) as resp:
                print(f'Created new GitHub repository: {username}/{repo_name}')
                return username, repo_name
        else:
            raise e

def sync_skills(repo_name='antigravity-skills', commit_msg=None):
    token = get_github_token()
    if not token:
        print('Error: GitHub Token not found in config or environment.')
        sys.exit(1)

    skills_dir = os.path.abspath(os.path.expanduser(r'~/.gemini/config/skills'))
    os.chdir(skills_dir)

    username, repo = ensure_github_repo(token, repo_name)
    remote_url = f'https://{token}@github.com/{username}/{repo}.git'

    # Inisialisasi git
    if not os.path.exists(os.path.join(skills_dir, '.git')):
        subprocess.run(['git', 'init'], check=True)
        subprocess.run(['git', 'branch', '-M', 'main'], check=True)
        subprocess.run(['git', 'remote', 'add', 'origin', remote_url], check=True)
    else:
        subprocess.run(['git', 'remote', 'set-url', 'origin', remote_url], check=True)

    # Set user config lokal jika belum ada
    subprocess.run(['git', 'config', 'user.name', username], check=True)
    subprocess.run(['git', 'config', 'user.email', f'{username}@users.noreply.github.com'], check=True)

    # Buat .gitignore
    gitignore_path = os.path.join(skills_dir, '.gitignore')
    if not os.path.exists(gitignore_path):
        with open(gitignore_path, 'w', encoding='utf-8') as f:
            f.write('__pycache__/\n*.pyc\n.env\n*.tmp\n')

    # Add all files
    subprocess.run(['git', 'add', '.'], check=True)

    # Cek status perubahan
    status_out = subprocess.run(['git', 'status', '--porcelain'], capture_output=True, text=True).stdout.strip()
    if not status_out:
        print('No new changes or skills to sync.')
        return

    # Deteksi nama skill
    new_skills = set()
    for line in status_out.splitlines():
        parts = line.strip().split()
        if len(parts) >= 2:
            path_parts = parts[-1].split('/')
            if len(path_parts) > 1:
                new_skills.add(path_parts[0])

    if not commit_msg:
        if new_skills:
            skill_list = ', '.join(sorted(list(new_skills))[:3])
            if len(new_skills) > 3:
                skill_list += f' and {len(new_skills)-3} more'
            commit_msg = f'feat(skills): sync verified skills ({skill_list})'
        else:
            commit_msg = 'chore(skills): update and synchronize verified skills'

    subprocess.run(['git', 'commit', '-m', commit_msg], check=True)
    
    # Push ke main
    push_res = subprocess.run(['git', 'push', '-u', 'origin', 'main', '--force'], capture_output=True, text=True)
    if push_res.returncode == 0:
        print(f'Successfully synchronized skills to https://github.com/{username}/{repo}')
    else:
        print(f'Git push notice: {push_res.stderr}')

if __name__ == '__main__':
    msg = sys.argv[1] if len(sys.argv) > 1 else None
    sync_skills(commit_msg=msg)
