/**
 * Conventional Commits configuration for Apalabrar.
 * See: https://www.conventionalcommits.org/
 */
module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'type-enum': [
      2,
      'always',
      [
        'feat', // new feature
        'fix', // bug fix
        'docs', // documentation only
        'style', // formatting, missing semicolons, etc
        'refactor', // refactor without behavior change
        'perf', // performance improvement
        'test', // adding/correcting tests
        'build', // build system, dependencies
        'ci', // CI configuration
        'chore', // routine task (no source/test change)
        'revert', // revert prior commit
        'release', // release commit
      ],
    ],
    'subject-case': [2, 'never', ['upper-case', 'pascal-case', 'start-case']],
    'subject-empty': [2, 'never'],
    'subject-full-stop': [2, 'never', '.'],
    'header-max-length': [2, 'always', 100],
    'body-leading-blank': [2, 'always'],
    'footer-leading-blank': [2, 'always'],
  },
};
